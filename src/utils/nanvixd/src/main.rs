// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod cache;
mod config;
mod http;
mod message;
mod sandbox;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    args::Args,
    cache::SandboxCache,
    http::HttpClient,
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

#[tokio::main]
pub async fn main() -> Result<()> {
    let args: Arc<Args> =
        Arc::new(Args::parse(std::env::args().filter(|s| !s.trim().is_empty()).collect())?);

    ::syslog::init(args.log_to_file(), args.log_directory().to_string());

    #[cfg(feature = "single-process")]
    info!("nanvixd {} single-process mode", env!("CARGO_PKG_VERSION"));
    #[cfg(not(feature = "single-process"))]
    info!("nanvixd {} multi-process mode", env!("CARGO_PKG_VERSION"));

    let mut signals: Signal = signal(SignalKind::interrupt())?;
    let http_listener: TcpListener = TcpListener::bind(args.http_sockaddr()).await?;
    let sandbox_cache: Arc<Mutex<SandboxCache>> = SandboxCache::new();

    loop {
        tokio::select! {
           result = http_listener.accept() => {
                match result {
                    Ok((stream, sockaddr)) => {
                        debug!("accepted connection from {sockaddr:?}");
                        let sandbox_cache_clone: Arc<Mutex<SandboxCache>> = sandbox_cache.clone();
                        let args_clone: Arc<Args> = args.clone();
                        // In single-process mode, handle connections sequentially.
                        #[cfg(feature = "single-process")]
                        {
                            let client: HttpClient = HttpClient::new(sandbox_cache_clone, args_clone);
                            let io: TokioIo<TcpStream> = TokioIo::new(stream);
                            if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                error!("failed to serve connection (error={e:?})");
                            }
                        }
                        #[cfg(not(feature = "single-process"))]
                        {
                            tokio::spawn(async move {
                                    let client: HttpClient = HttpClient::new(sandbox_cache_clone, args_clone);
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
                break;
            },
        }
    }

    Ok(())
}
