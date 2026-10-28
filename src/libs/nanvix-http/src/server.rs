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
use ::std::sync::Arc;
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
    /// cleaning up all active sandboxes.
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

        loop {
            tokio::select! {
            result = http_listener.accept() => {
                    match result {
                        Ok((stream, sockaddr)) => {
                            debug!("accepted connection from {sockaddr:?}");
                            // Disable Nagle's algorithm so small HTTP responses are sent immediately
                            // instead of being delayed up to 40 ms by the TCP delayed-ACK interaction.
                            if let Err(e) = stream.set_nodelay(true) {
                                error!("failed to set TCP_NODELAY (error={e:?})");
                            }
                            // Handle each connection sequentially.
                            let client: HttpClient = HttpClient::new(state.clone());
                            let io: TokioIo<TcpStream> = TokioIo::new(stream);
                            if let Err(e) = http1::Builder::new()
                                .serve_connection(io, client)
                                .await
                            {
                                error!("failed to serve connection (error={e:?})");
                            }
                        },
                        Err(e) => {
                            error!("failed to accept connection ({e:?})");
                        },
                    }
                },
                signal = shutdown_signal.recv() => {
                    info!("received {signal}, stopping...");
                    let has_vm: bool = state.has_running_vm().await;
                    info!("shutdown snapshot: has_running_vm={has_vm}");
                    state.cleanup().await;
                    break Ok(());
                },
            }
        }
    }
}
