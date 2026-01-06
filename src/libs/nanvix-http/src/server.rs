// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP server implementation for Nanvix Daemon.
//!
//! This module provides the HTTP server that listens for incoming client connections and
//! dispatches requests to appropriate handlers. It manages the server lifecycle, handles
//! graceful shutdown on interrupt signals, and maintains the sandbox cache for all active
//! instances.

//==================================================================================================
// Imports
//==================================================================================================

use crate::client::HttpClient;
use ::anyhow::Result;
use ::hyper::server::conn::http1;
use ::hyper_util::rt::TokioIo;
use ::nanvix_sandbox_cache::{
    SandboxCache,
    SandboxCacheConfig,
    SandboxCacheStateSummary,
};
use ::std::sync::Arc;
use ::syslog::{
    debug,
    error,
    info,
};
use ::tokio::{
    net::{
        TcpListener,
        TcpStream,
    },
    signal::unix::{
        signal,
        Signal,
        SignalKind,
    },
    sync::Mutex,
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
/// handles interrupt signals for graceful shutdown, and maintains the sandbox cache for
/// all active instances. It provides the main event loop for the daemon.
///
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Must implement `Send + Sync + Default + Clone`. Use `()` if no custom
///   state is required.
///
pub struct HttpServer<T> {
    /// Socket address to bind the HTTP server to.
    sockaddr: String,
    /// Configuration for sandbox cache management.
    config: SandboxCacheConfig<T>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Send + Sync + Default + Clone + 'static> HttpServer<T> {
    ///
    /// # Description
    ///
    /// Creates a new HTTP server with the specified configuration.
    ///
    /// # Parameters
    ///
    /// - `sockaddr`: Socket address (host:port) to bind the server to.
    /// - `config`: Configuration parameters for sandbox cache management.
    ///
    /// # Returns
    ///
    /// A new HTTP server instance ready to be started.
    ///
    pub fn new(sockaddr: &str, config: SandboxCacheConfig<T>) -> Self {
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
    /// them to HTTP client handlers. In single-process mode, connections are handled sequentially.
    /// In multi-process mode, each connection is handled in a separate tokio task.
    ///
    /// The server runs until an interrupt signal (SIGINT) is received, at which point it performs
    /// graceful shutdown by cleaning up all active sandboxes.
    ///
    /// # Returns
    ///
    /// On success, returns an empty tuple after graceful shutdown. On failure, returns an error
    /// describing what went wrong during server operation.
    ///
    pub async fn run(&mut self) -> Result<()> {
        let mut signals: Signal = signal(SignalKind::interrupt())?;
        let http_listener: TcpListener = TcpListener::bind(&self.sockaddr).await?;
        let sandbox_cache: Arc<Mutex<SandboxCache<T>>> = SandboxCache::new(self.config.clone())?;

        loop {
            tokio::select! {
            result = http_listener.accept() => {
                    match result {
                        Ok((stream, sockaddr)) => {
                            debug!("accepted connection from {sockaddr:?}");
                            let sandbox_cache_clone: Arc<Mutex<SandboxCache<T>>> = sandbox_cache.clone();
                            // In single-process mode, handle connections sequentially.
                            #[cfg(feature = "single-process")]
                            {
                                let client: HttpClient<T> = HttpClient::new(sandbox_cache_clone);
                                let io: TokioIo<TcpStream> = TokioIo::new(stream);
                                if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                    error!("failed to serve connection (error={e:?})");
                                }
                            }
                            #[cfg(not(feature = "single-process"))]
                            {
                                tokio::spawn(async move {
                                    let client: HttpClient<T> = HttpClient::new(sandbox_cache_clone);
                                    let io: TokioIo<TcpStream> = TokioIo::new(stream);
                                    if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                        error!("failed to serve connection (error={e:?})");
                                    }
                                });
                            }
                        },
                        Err(e) => {
                            error!("failed to accept connection ({e:?})");
                        },
                    }
                },
                _ = signals.recv() => {
                    info!("received exit signal, stopping...");
                    let sandbox_cache_clone: Arc<Mutex<SandboxCache<T>>> = sandbox_cache.clone();
                    let mut cache_guard = sandbox_cache_clone.lock().await;
                    let summary: SandboxCacheStateSummary = cache_guard.state_summary();
                    info!(
                        "shutdown snapshot: running_sandboxes={}, linuxd_instances={}, sandbox_index_entries={}, \
                         control_plane_socket={}, l2_enabled={}",
                        summary.running_sandboxes(),
                        summary.linuxd_instances(),
                        summary.sandbox_index_entries(),
                        summary.has_control_plane_socket(),
                        summary.l2_enabled()
                    );
                    cache_guard.cleanup().await;
                    break Ok(());
                },
            }
        }
    }
}
