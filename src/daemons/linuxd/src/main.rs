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
mod control_plane;
mod dirent;
mod error;
mod fcntl;
mod linuxd;
mod message;
mod poll;
mod socket;
mod time;
mod times;
mod unistd;
mod user_vm_handle;
mod venv;
mod worker_thread;

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
use ::std::{
    env,
    str::FromStr,
    sync::Once,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
};
use ::syscomm::{
    Socket,
    SocketListener,
    SocketStream,
    SocketType,
};
use sys::pm::ThreadIdentifier;

//==================================================================================================
// Constants
//==================================================================================================

/// Default control-plane socket type.
const DEFAULT_CONTROL_PLANE_SOCKET_TYPE: SocketType = SocketType::Unix;

/// Default user VM type.
const DEFAULT_USER_VM_SOCKET_TYPE: SocketType = SocketType::Unix;

/// Default gateway socket type.
const DEFAULT_GATEWAY_SOCKET_TYPE: SocketType = SocketType::Unix;

//==================================================================================================
// Implementations
//==================================================================================================

pub fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    initialize(args.log_to_file());

    // Work-out the socket addresses.
    let control_plane_sockaddr: String = args.control_plane_sockaddr();
    let user_vm_sockaddr: String = args.user_vm_bind_sockaddr();
    let gateway_sockaddr: Option<String> = args.gateway_bind_sockaddr();

    // Work-out the socket-types.
    let control_plane_socket_type: SocketType = match args.control_plane_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={typ:?})");
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_CONTROL_PLANE_SOCKET_TYPE,
    };

    let user_vm_bind_socket_type: SocketType = match args.user_vm_bind_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={typ:?})");
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_USER_VM_SOCKET_TYPE,
    };

    let gateway_bind_socket_type: SocketType = match args.gateway_bind_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={typ:?})");
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_GATEWAY_SOCKET_TYPE,
    };

    // Connect the control-plane socket.
    let control_plane_stream: SocketStream =
        match SocketStream::connect(control_plane_socket_type, control_plane_sockaddr.clone()) {
            Ok(socket) => {
                info!("Connected to control plane on: {:?}", control_plane_sockaddr);
                socket
            },
            Err(e) => {
                error!(
                    "failed to connect to control-plane socket address (address={}, error={e:?})",
                    control_plane_sockaddr.clone()
                );
                anyhow::bail!("failed to connect to control-plane socket address");
            },
        };

    // Start listening for incoming connections from user VMs associated to this linuxd instance.
    let user_vm_listener: SocketListener =
        match Socket::bind(user_vm_bind_socket_type, user_vm_sockaddr.clone()) {
            Ok(listener) => listener,
            Err(e) => {
                error!(
                    "failed to bind to user VM socket address (address={}, error={e:?})",
                    user_vm_sockaddr.clone()
                );
                anyhow::bail!("failed to bind to user VM socket address");
            },
        };
    info!("Listening to user VMs on: {user_vm_sockaddr:?}");

    // Start listening for requests for requests to feed input to the user VM.
    let gateway_listener: Option<SocketListener> = match gateway_sockaddr {
        Some(ref sockaddr) => match Socket::bind(gateway_bind_socket_type, sockaddr.clone()) {
            Ok(stream) => {
                info!("Listening for user VM input on gateway at: {:?}", gateway_sockaddr);
                Some(stream)
            },
            Err(e) => {
                error!("failed to bind to gateway (address={}, error={e:?})", sockaddr.clone());
                anyhow::bail!("failed to bind to gateway");
            },
        },
        None => None,
    };

    let procd: LinuxDaemon =
        match LinuxDaemon::init(control_plane_stream, user_vm_listener, gateway_listener) {
            Ok(procd) => procd,
            Err(e) => panic!("failed to initialize process manager daemon (error={e:?})"),
        };

    // Run main procd loop.
    let procd_ret = procd.run();

    // Do not panic here as we have already exited the loop. Instead, continue with clean-up.
    if procd_ret.is_err() {
        error!("error running procd (error={procd_ret:?})");
    }

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
        let logger =
            Logger::try_with_env_or_str("error").expect("malformed RUST_LOG environment variable");
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
/// - `tid`: Thread identifier.
/// - `error`: Error code.
///
/// # Returns
///
/// A message with the error response.
///
pub fn build_error(tid: ThreadIdentifier, error: ErrorCode) -> Message {
    Message::new(
        MessageSender::from(::syscall::LINUXD),
        MessageReceiver::from(tid),
        MessageType::Ikc,
        Some(error),
        [0u8; Message::PAYLOAD_SIZE],
    )
}
