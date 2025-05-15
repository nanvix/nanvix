// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![feature(str_from_raw_parts)] // dirent requires this.

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod dirent;
mod fcntl;
mod linuxd;
mod message;
mod socket;
mod time;
mod times;
mod unistd;
mod venv;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

extern crate alloc;

use self::{
    args::Args,
    linuxd::LinuxDaemon,
};
use ::anyhow::Result;
use ::flexi_logger::{
    FileSpec,
    Logger,
};
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};
use ::signal_hook::{
    consts::SIGINT,
    iterator::{
        Signals,
        SignalsInfo,
    },
};
use ::std::{
    env,
    fs,
    os::unix::net::UnixStream,
    str::FromStr,
    sync::Once,
    thread,
};
use ::syscomm::{
    Socket,
    SocketListener,
    SocketStream,
    SocketType,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default socket bind type.
const DEFAULT_BIND_SOCKET_TYPE: SocketType = SocketType::Unix;

//==================================================================================================
// Implementations
//==================================================================================================

pub fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    let sockaddr: String = args.bind_sockaddr();
    initialize(args.log_to_file());

    let bind_socket_type: SocketType = match args.bind_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={:?})", typ);
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_BIND_SOCKET_TYPE,
    };

    let listener: SocketListener = match Socket::bind(bind_socket_type, sockaddr.clone()) {
        Ok(listener) => listener,
        Err(e) => {
            error!("failed to bind to socket address (error={:?})", e);
            anyhow::bail!("failed to bind to socket address");
        },
    };

    // Install signal handler.
    let path: Option<String> = match bind_socket_type {
        SocketType::Tcp => None,
        SocketType::Unix => Some(sockaddr.clone()),
    };
    let mut signals: SignalsInfo = Signals::new([SIGINT])?;
    thread::spawn(move || {
        #[allow(clippy::never_loop)]
        for sig in signals.forever() {
            println!("Received signal {:?}", sig);
            if let Some(path) = path {
                if let Err(e) = fs::remove_file(path.clone()) {
                    error!("failed to remove socket file (error={:?})", e);
                }
            }
            // Exit process.
            std::process::exit(0);
        }
    });

    loop {
        // Connect to gateway after binding to socket address, as a connection to the gateway will
        // signal we are ready to accept commands.
        let mut gateway_conn: Option<UnixStream> = match args.gateway_sockaddr() {
            Some(sockaddr) => match UnixStream::connect(sockaddr) {
                Ok(stream) => Some(stream),
                Err(e) => {
                    error!("failed to connect to gateway (error={:?})", e);
                    anyhow::bail!("failed to connect to gateway");
                },
            },
            None => None,
        };

        info!("Listening on: {:?}", sockaddr);
        let stream: SocketStream = match listener.accept() {
            Ok(stream) => {
                info!("Connected to: {:?}", stream.peer_addr());
                stream
            },
            Err(error) => {
                error!("Failed to accept connection: {:?}", error);
                continue;
            },
        };

        let mut procd: LinuxDaemon = match LinuxDaemon::init(stream, &mut gateway_conn) {
            Ok(procd) => procd,
            Err(e) => panic!("failed to initialize process manager daemon (error={:?})", e),
        };

        if procd.run().is_err() {
            break;
        }
    }

    fs::remove_file(sockaddr)?;

    Ok(())
}

///
/// # Description
///
/// Initializes the logger.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn initialize(logfile: bool) {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        let logger = Logger::try_with_env().expect("malformed RUST_LOG environment variable");
        if logfile {
            logger
                .log_to_file(FileSpec::default())
                .start()
                .expect("failed to initialize logger");
        } else {
            logger.start().expect("failed to initialize logger");
        }
    });
}

///
/// # Description
///
/// Builds an error response message.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `error`: Error code.
///
/// # Returns
///
/// A message with the error response.
///
pub fn build_error(pid: ProcessIdentifier, error: ErrorCode) -> Message {
    Message::new(::posix::LINUXD, pid, MessageType::Ikc, Some(error), [0u8; Message::PAYLOAD_SIZE])
}
