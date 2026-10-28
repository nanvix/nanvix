// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP server implementation for Nanvix Daemon.
//!
//! This module provides the HTTP server that listens for incoming client connections and
//! dispatches requests to appropriate handlers. It manages the server lifecycle, handles
//! graceful shutdown on shutdown signals, and maintains the sandbox cache for all active
//! instances.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    client::HttpClient,
    signal::ShutdownSignal,
    StandaloneState,
};
use ::anyhow::Result;
use ::hyper::server::conn::http1;
use ::hyper_util::rt::TokioIo;
use ::log::{
    debug,
    error,
    info,
};
use ::nanvix_sandbox_config::StandaloneConfig;
use ::std::{
    future::Future,
    net::SocketAddr,
    sync::Arc,
};
use ::tokio::net::{
    TcpListener,
    TcpStream,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// HTTP server for the Nanvix Daemon.
///
/// This structure manages the HTTP server that listens for incoming client connections,
/// handles shutdown signals for graceful shutdown, and maintains the sandbox cache for
/// all active instances. It provides the main event loop for the daemon.
///
pub struct HttpServer {
    /// Socket address to bind the HTTP server to.
    sockaddr: String,
    /// Configuration for standalone VM management.
    config: StandaloneConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl HttpServer {
    ///
    /// # Description
    ///
    /// Creates a new HTTP server with the specified standalone configuration.
    ///
    /// # Parameters
    ///
    /// - `sockaddr`: Socket address (host:port) to bind the server to.
    /// - `config`: Standalone configuration with kernel and VM parameters.
    ///
    /// # Returns
    ///
    /// A new HTTP server instance ready to be started.
    ///
    pub fn new(sockaddr: &str, config: StandaloneConfig) -> Self {
        Self {
            sockaddr: sockaddr.to_string(),
            config,
        }
    }

    ///
    /// # Description
    ///
    /// Runs the HTTP server's main event loop.
    ///
    /// This method binds to the configured address, accepts incoming connections, and dispatches
    /// them to HTTP client handlers. Connections are handled sequentially.
    ///
    /// The server runs until a shutdown signal is received (`SIGINT`, `SIGTERM`, `SIGHUP`, or
    /// `SIGQUIT` on Unix, Ctrl-C on Windows), at which point it performs graceful shutdown by
    /// cleaning up all active sandboxes. The shutdown signal is honored even while a connection
    /// is being served, in which case the connection is closed and its in-flight request is
    /// cancelled.
    ///
    /// # Returns
    ///
    /// On success, returns an empty tuple after graceful shutdown. On failure, returns an error
    /// describing what went wrong during server operation.
    ///
    pub async fn run(&mut self) -> Result<()> {
        // Initialize shared state and install shutdown signal handlers before binding the socket,
        // as some setups may use socket readiness to probe nanvixd's readiness, and they may stop
        // nanvixd right after it becomes ready.
        let state: Arc<StandaloneState> = Arc::new(StandaloneState::new(self.config.clone()));
        let mut shutdown_signal: ShutdownSignal = ShutdownSignal::new()?;
        let http_listener: TcpListener = TcpListener::bind(&self.sockaddr).await?;

        serve(http_listener, state, shutdown_signal.recv()).await;

        Ok(())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Serves incoming connections until a shutdown signal is received, and then performs graceful
/// shutdown by cleaning up all active sandboxes.
///
/// Connections are handled sequentially, but the shutdown signal is watched while a connection is
/// being served, as well as while waiting for one. Serving a connection may take arbitrarily long:
/// a client may keep an idle connection alive, and a KILL request waits for the VM to exit, which
/// it may never do on its own. If the shutdown signal arrives while a connection is being served,
/// the connection is closed and its in-flight request is cancelled.
///
/// # Parameters
///
/// - `http_listener`: Listener that accepts client connections.
/// - `state`: Shared standalone state.
/// - `shutdown_signal`: Future that completes with the name of the shutdown signal once it is
///   received.
///
async fn serve(
    http_listener: TcpListener,
    state: Arc<StandaloneState>,
    shutdown_signal: impl Future<Output = &'static str>,
) {
    ::tokio::pin!(shutdown_signal);

    let signal: &str = loop {
        let (stream, sockaddr): (TcpStream, SocketAddr) = tokio::select! {
            result = http_listener.accept() => match result {
                Ok(connection) => connection,
                Err(e) => {
                    error!("failed to accept connection ({e:?})");
                    continue;
                },
            },
            signal = &mut shutdown_signal => break signal,
        };

        debug!("accepted connection from {sockaddr:?}");
        // Disable Nagle's algorithm so small HTTP responses are sent immediately
        // instead of being delayed up to 40 ms by the TCP delayed-ACK interaction.
        if let Err(e) = stream.set_nodelay(true) {
            error!("failed to set TCP_NODELAY (error={e:?})");
        }
        // Handle each connection sequentially.
        let client: HttpClient = HttpClient::new(state.clone());
        let io: TokioIo<TcpStream> = TokioIo::new(stream);
        tokio::select! {
            result = http1::Builder::new().serve_connection(io, client) => {
                if let Err(e) = result {
                    error!("failed to serve connection (error={e:?})");
                }
            },
            signal = &mut shutdown_signal => {
                info!("closing connection from {sockaddr:?} to shut down");
                break signal;
            },
        }
    };

    info!("received {signal}, stopping...");
    let has_vm: bool = state.has_running_vm().await;
    info!("shutdown snapshot: has_running_vm={has_vm}");
    state.cleanup().await;
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use crate::message::{
        self,
        MessageType,
        HTTP_HEADER_MESSAGE_TYPE,
    };
    use ::nanvix_sandbox_config::{
        HostFilter,
        NetworkingMode,
    };
    use ::std::{
        path::{
            Path,
            PathBuf,
        },
        time::Duration,
    };
    use ::tempfile::TempDir;
    use ::tokio::{
        io::{
            AsyncReadExt,
            AsyncWriteExt,
        },
        sync::oneshot,
        task::JoinHandle,
        time::timeout,
    };

    /// Maximum time to wait for the server to shut down.
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

    /// Name of the shutdown signal that the tests deliver to the server.
    const TEST_SIGNAL_NAME: &str = "test signal";

    /// Readiness probe, sent over a persistent connection.
    const READY_REQUEST: &[u8] = b"GET /ready HTTP/1.1\r\nHost: localhost\r\n\r\n";

    /// Server under test.
    struct TestServer {
        /// Address on which the server accepts connections.
        sockaddr: SocketAddr,
        /// Sender that delivers the shutdown signal to the server.
        shutdown_tx: oneshot::Sender<()>,
        /// Task running the server.
        task: JoinHandle<()>,
    }

    impl TestServer {
        /// Starts a server with the given standalone configuration on an ephemeral port.
        async fn start(config: StandaloneConfig) -> Self {
            let http_listener: TcpListener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("failed to bind HTTP listener");
            let sockaddr: SocketAddr = http_listener
                .local_addr()
                .expect("failed to get HTTP listener address");
            let state: Arc<StandaloneState> = Arc::new(StandaloneState::new(config));
            let (shutdown_tx, shutdown_rx): (oneshot::Sender<()>, oneshot::Receiver<()>) =
                oneshot::channel();
            let task: JoinHandle<()> = ::tokio::spawn(serve(http_listener, state, async move {
                let _ = shutdown_rx.await;
                TEST_SIGNAL_NAME
            }));
            Self {
                sockaddr,
                shutdown_tx,
                task,
            }
        }

        /// Delivers the shutdown signal, and waits for the server to shut down.
        async fn shut_down(self) {
            self.shutdown_tx
                .send(())
                .expect("server stopped before it was asked to");
            timeout(SHUTDOWN_TIMEOUT, self.task)
                .await
                .expect("server did not shut down in time")
                .expect("server task failed");
        }
    }

    /// Builds a configuration whose VMs fail to boot right away, as their kernel does not exist.
    fn test_config(tmp_dir: &Path, gateway_sockaddr: Option<String>) -> StandaloneConfig {
        let kernel_binary_path: PathBuf = tmp_dir.join("missing-kernel.elf");
        StandaloneConfig::new(
            kernel_binary_path.to_string_lossy().into_owned(),
            None,
            None,
            None,
            None,
            None,
            NetworkingMode::Disabled,
            HostFilter::default(),
            #[cfg(feature = "gdb")]
            None,
            gateway_sockaddr,
        )
    }

    /// Reads the head of an HTTP response, which ends with an empty line.
    async fn read_response_head(stream: &mut TcpStream) -> String {
        let mut head: Vec<u8> = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte: [u8; 1] = [0u8; 1];
            stream
                .read_exact(&mut byte)
                .await
                .expect("failed to read response head");
            head.push(byte[0]);
        }
        String::from_utf8_lossy(&head).into_owned()
    }

    #[tokio::test]
    async fn serve_shuts_down_while_serving_a_connection() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
        let server: TestServer = TestServer::start(test_config(tmp_dir.path(), None)).await;

        // Complete a request on a persistent connection, so that the server is still serving it
        // when the shutdown signal arrives.
        let mut stream: TcpStream = TcpStream::connect(server.sockaddr)
            .await
            .expect("failed to connect to server");
        stream
            .write_all(READY_REQUEST)
            .await
            .expect("failed to send readiness probe");
        let head: String = read_response_head(&mut stream).await;
        assert!(head.starts_with("HTTP/1.1 204"), "unexpected response: {head}");

        server.shut_down().await;

        // The server closes the connection as it shuts down.
        let mut buffer: [u8; 1] = [0u8; 1];
        let len: usize = stream.read(&mut buffer).await.unwrap_or(0);
        assert_eq!(len, 0, "connection should be closed");
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn serve_removes_gateway_socket_on_shutdown() {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
        let gateway_path: PathBuf = tmp_dir.path().join("gateway.sock");
        let gateway_sockaddr: String = gateway_path.to_string_lossy().into_owned();
        let server: TestServer =
            TestServer::start(test_config(tmp_dir.path(), Some(gateway_sockaddr.clone()))).await;

        // Start a VM. The gateway endpoint is bound before the VM boots, and it stays bound until
        // the VM is torn down, even though this VM fails to boot.
        let new_message: message::New = message::New {
            tenant_id: "tenant".to_string(),
            app_name: "app".to_string(),
            program: tmp_dir
                .path()
                .join("program.elf")
                .to_string_lossy()
                .into_owned(),
            program_args: String::new(),
        };
        let body: String = ::serde_json::to_string(&new_message).expect("failed to encode NEW");
        let request: String = format!(
            "POST / HTTP/1.1\r\nHost: localhost\r\n{HTTP_HEADER_MESSAGE_TYPE}: \
             {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
             close\r\n\r\n{body}",
            MessageType::New,
            body.len()
        );
        let mut stream: TcpStream = TcpStream::connect(server.sockaddr)
            .await
            .expect("failed to connect to server");
        stream
            .write_all(request.as_bytes())
            .await
            .expect("failed to send NEW request");
        let mut response: String = String::new();
        stream
            .read_to_string(&mut response)
            .await
            .expect("failed to read NEW response");
        assert!(response.starts_with("HTTP/1.1 200"), "unexpected response: {response}");
        assert!(response.contains(&gateway_sockaddr), "unexpected response: {response}");
        assert!(gateway_path.exists(), "gateway socket should exist while the VM is running");

        server.shut_down().await;

        assert!(!gateway_path.exists(), "gateway socket should be removed on shutdown");
    }
}
