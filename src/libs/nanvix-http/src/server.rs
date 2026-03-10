// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP server implementation for Nanvix Daemon.
//!
//! This module provides the HTTP server that listens for incoming client connections and
//! dispatches requests to appropriate handlers. It manages the server lifecycle, handles
//! graceful shutdown on interrupt signals, and maintains the sandbox cache for all active
//! instances.

#[cfg(all(feature = "single-process", feature = "standalone"))]
compile_error!("features `single-process` and `standalone` are mutually exclusive");

//==================================================================================================
// Imports
//==================================================================================================

use crate::client::HttpClient;
#[cfg(feature = "standalone")]
use crate::{
    StandaloneConfig,
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
#[cfg(feature = "single-process")]
use ::nanvix_sandbox::simple_cache::SimpleSandboxCache;
#[cfg(feature = "single-process")]
use ::nanvix_sandbox::simple_cache::SimpleSandboxCacheConfig;
#[cfg(not(any(feature = "single-process", feature = "standalone")))]
use ::nanvix_sandbox_cache::{
    SandboxCache,
    SandboxCacheConfig,
    SandboxCacheStateSummary,
};
use ::std::sync::Arc;
#[cfg(feature = "single-process")]
use ::tokio::sync::Mutex;
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
    #[cfg(feature = "single-process")]
    config: SimpleSandboxCacheConfig<T>,
    #[cfg(feature = "standalone")]
    config: StandaloneConfig,
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
    config: SandboxCacheConfig<T>,
    #[cfg(feature = "standalone")]
    _phantom: ::std::marker::PhantomData<T>,
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
    #[cfg(feature = "single-process")]
    pub fn new(sockaddr: &str, config: SimpleSandboxCacheConfig<T>) -> Self {
        Self {
            sockaddr: sockaddr.to_string(),
            config,
        }
    }

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
    #[cfg(feature = "standalone")]
    pub fn new(sockaddr: &str, config: StandaloneConfig) -> Self {
        Self {
            sockaddr: sockaddr.to_string(),
            config,
            _phantom: ::std::marker::PhantomData,
        }
    }

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
    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
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
        // Initialize shared state before binding the socket, as some setups may use socket
        // readiness to probe nanvixd's readiness.
        #[cfg(feature = "single-process")]
        let sandbox_cache: Arc<Mutex<SimpleSandboxCache<T>>> =
            SimpleSandboxCache::new(self.config.clone()).await?;
        // In standalone mode, bypass the sandbox cache and drive User VMs directly.
        #[cfg(feature = "standalone")]
        let sandbox_cache: Arc<StandaloneState> =
            Arc::new(StandaloneState::new(self.config.clone()));
        #[cfg(not(any(feature = "single-process", feature = "standalone")))]
        let sandbox_cache: Arc<SandboxCache<T>> = SandboxCache::new(self.config.clone()).await?;
        let mut signals: Signal = signal(SignalKind::interrupt())?;
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
                            // In single-process and standalone mode, handle connections sequentially.
                            #[cfg(any(feature = "single-process", feature = "standalone"))]
                            {
                                let client: HttpClient<T> =
                                    HttpClient::new(sandbox_cache.clone());
                                let io: TokioIo<TcpStream> = TokioIo::new(stream);
                                if let Err(e) = http1::Builder::new()
                                    .serve_connection(io, client)
                                    .await
                                {
                                    error!("failed to serve connection (error={e:?})");
                                }
                            }
                            // In multi-process mode, spawn a new task for each connection.
                            #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                            {
                                let sandbox_cache_clone = sandbox_cache.clone();
                                tokio::spawn(async move {
                                    let client: HttpClient<T> =
                                        HttpClient::new(sandbox_cache_clone);
                                    let io: TokioIo<TcpStream> = TokioIo::new(stream);
                                    if let Err(e) = http1::Builder::new()
                                        .serve_connection(io, client)
                                        .await
                                    {
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
                    #[cfg(feature = "single-process")]
                    {
                        let mut cache_guard = sandbox_cache.lock().await;
                        let summary = cache_guard.state_summary();
                        info!(
                            "shutdown snapshot: has_running_sandbox={}, linuxd_instances={}, \
                             control_plane_socket={}",
                            summary.has_running_sandbox(),
                            summary.linuxd_instances(),
                            summary.has_control_plane_bind_socket(),
                        );
                        cache_guard.cleanup().await;
                    }
                    #[cfg(feature = "standalone")]
                    {
                        let has_vm: bool = sandbox_cache.has_running_vm().await;
                        info!("shutdown snapshot: has_running_vm={has_vm}");
                        sandbox_cache.cleanup().await;
                    }
                    #[cfg(not(any(feature = "single-process", feature = "standalone")))]
                    {
                        let summary: SandboxCacheStateSummary = sandbox_cache.state_summary().await;
                        info!(
                            "shutdown snapshot: running_sandboxes={}, linuxd_instances={}, \
                             l2_enabled={}",
                            summary.running_sandboxes(),
                            summary.linuxd_instances(),
                            summary.l2_enabled()
                        );
                        sandbox_cache.cleanup().await;
                    }
                    break Ok(());
                },
            }
        }
    }
}
