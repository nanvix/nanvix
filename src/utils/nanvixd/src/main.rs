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
mod control_plane;
mod http;
mod logging;
mod message;
mod sandbox;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

use crate::{
    args::Args,
    cache::SandboxCache,
    http::HttpClient,
};
use ::anyhow::Result;
use ::hwloc::HwLoc;
use ::hyper::server::conn::http1;
use ::hyper_util::rt::TokioIo;
use ::std::sync::Arc;
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
    logging::initialize();

    let args: Args = Args::parse(std::env::args().collect())?;

    let mut signals: Signal = signal(SignalKind::interrupt())?;
    let http_listener: TcpListener = TcpListener::bind(args.http_sockaddr()).await?;
    let sandbox_cache: Arc<Mutex<SandboxCache>> = SandboxCache::new();

    loop {
        tokio::select! {
           result = http_listener.accept() => {
                match result {
                    Ok((stream, sockaddr)) => {
                        debug!("accepted connection from {sockaddr:?}");
                        let tmp_directory: String = args.tmp_directory().to_string();
                        let binary_directory: String = args.binary_directory().to_string();
                        let console_file: Option<String> = args.nanvix_console();
                        let hwloc: Option<HwLoc> = args.hwloc();
                        let sandboxe_cache: Arc<Mutex<SandboxCache>> = sandbox_cache.clone();
                        tokio::spawn(async move {
                            let client =
                                HttpClient::new(sandboxe_cache, tmp_directory, binary_directory, console_file, hwloc);
                            let io: TokioIo<TcpStream> = TokioIo::new(stream);
                            if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                error!("failed to serve connection (error={e:?})");
                            }
                        });
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
