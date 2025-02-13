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
mod logging;
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
use ::hyper::server::conn::http1;
use ::hyper_util::rt::TokioIo;
use ::std::{
    fs,
    sync::{
        atomic::AtomicUsize,
        Arc,
    },
};
use ::tokio::{
    net::{
        TcpListener,
        TcpStream,
        UnixListener,
        UnixSocket,
    },
    signal::unix::{
        signal,
        Signal,
        SignalKind,
    },
};

//==================================================================================================

#[tokio::main]
pub async fn main() -> Result<()> {
    logging::initialize();

    let args: Args = Args::parse(std::env::args().collect())?;
    let sandbox_sockaddr: String = config::sandbox_sockaddr_builder(args.sandbox_sockaddr());

    let mut signals: Signal = signal(SignalKind::interrupt())?;
    let http_listener: TcpListener = TcpListener::bind(args.http_sockaddr()).await?;
    let socket: UnixSocket = UnixSocket::new_stream()?;
    socket.bind(&sandbox_sockaddr)?;
    let linuxd_listener: Arc<UnixListener> =
        Arc::new(socket.listen(config::LINUXD_SOCKET_BACKLOG)?);
    let requestid: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let sandbox_cache: SandboxCache = SandboxCache::new(args.keep_alive_timeout());

    loop {
        tokio::select! {
           result = http_listener.accept() => {
                match result {
                    Ok((stream, sockaddr)) => {
                        debug!("accepted connection from {:?}", sockaddr);
                        let linuxd_listener: Arc<UnixListener> = linuxd_listener.clone();
                        let linuxd_sockaddr: String = args.linuxd_sockaddr().to_string();
                        let sandbox_sockaddr: String = sandbox_sockaddr.clone();
                        let console_file: String = args.nanvix_console().to_string();
                        let requestid: Arc<AtomicUsize> = requestid.clone();
                        let sandboxe_cache: SandboxCache = sandbox_cache.clone();
                        tokio::spawn(async move {
                            let requestid: usize = requestid
                                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            let client =
                                HttpClient::new(sandboxe_cache, requestid, linuxd_listener, linuxd_sockaddr, sandbox_sockaddr, console_file);
                            let io: TokioIo<TcpStream> = TokioIo::new(stream);
                            if let Err(e) = http1::Builder::new().serve_connection(io, client).await  {
                                error!("failed to serve connection (requestid={:?}, error={:?})", requestid, e);
                            }
                        });
                    },
                    Err(e) => {
                        error!("failed to accept connection ({:?})", e);
                    },
                }
            },
            _ = sandbox_cache.try_cleanup() => {
            },
            _ = signals.recv() => {
                info!("received exit signal, stopping...");
                break;
            },
        }
    }

    if let Err(e) = fs::remove_file(&sandbox_sockaddr) {
        error!("failed to remove socket file ({:?})", e);
    }

    Ok(())
}
