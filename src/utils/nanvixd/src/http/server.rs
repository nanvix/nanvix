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

use crate::{
    cache::{
        config::SandboxCacheConfig,
        SandboxCache,
    },
    http::client::HttpClient,
};
use ::anyhow::Result;
use ::hyper::server::conn::http1;
use ::hyper_util::rt::TokioIo;
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
pub struct HttpServer {
    /// Socket address to bind the HTTP server to.
    sockaddr: String,
    /// Configuration for sandbox cache management.
    config: SandboxCacheConfig,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl HttpServer {
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
    pub fn new(sockaddr: &str, config: SandboxCacheConfig) -> Self {
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
        let sandbox_cache: Arc<Mutex<SandboxCache>> = SandboxCache::new(self.config.clone());

        loop {
            tokio::select! {
            result = http_listener.accept() => {
                    match result {
                        Ok((stream, sockaddr)) => {
                            debug!("accepted connection from {sockaddr:?}");
                            let sandbox_cache_clone: Arc<Mutex<SandboxCache>> = sandbox_cache.clone();
                            // In single-process mode, handle connections sequentially.
                            #[cfg(feature = "single-process")]
                            {
                                let client: HttpClient = HttpClient::new(sandbox_cache_clone);
                                let io: TokioIo<TcpStream> = TokioIo::new(stream);
                                if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                    error!("failed to serve connection (error={e:?})");
                                }
                            }
                            #[cfg(not(feature = "single-process"))]
                            {
                                tokio::spawn(async move {
                                    let client: HttpClient = HttpClient::new(sandbox_cache_clone);
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
                    sandbox_cache
                        .clone()
                        .lock()
                        .await
                        .cleanup()
                        .await;
                    break Ok(());
                },
            }
        }
    }
}
