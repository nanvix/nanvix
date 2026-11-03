// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
        "POST / HTTP/1.1\r\nHost: localhost\r\n{HTTP_HEADER_MESSAGE_TYPE}: {}\r\nContent-Type: \
         application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
