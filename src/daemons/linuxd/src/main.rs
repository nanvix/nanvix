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
mod config;
mod dirent;
mod error;
mod fcntl;
mod linuxd;
mod message;
mod poll;
mod socket;
mod sys_select;
mod time;
mod times;
mod unistd;
mod user_vm_handle;
mod venv;
mod worker_thread;

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use self::{
    args::Args,
    linuxd::LinuxDaemon,
};
use ::anyhow::Result;
use ::std::{
    env,
    str::FromStr,
};
use ::sys::{
    error::ErrorCode,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use ::syscomm::{
    Socket,
    SocketListener,
    SocketType,
};
use ::syslog::{
    error,
    info,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default control-plane socket type.
const DEFAULT_CONTROL_PLANE_SOCKET_TYPE: SocketType = SocketType::Unix;

/// Default user VM type.
const DEFAULT_USER_VM_SOCKET_TYPE: SocketType = SocketType::Unix;

//==================================================================================================
// Implementations
//==================================================================================================

pub fn main() -> Result<()> {
    // Parse and retrieve command-line arguments.
    let args: Args = args::Args::parse(env::args().collect())?;
    ::syslog::init(args.log_to_file(), args.log_file_dir());

    // Work-out the socket addresses.
    let control_plane_sockaddr: String = args.control_plane_sockaddr();
    let user_vm_sockaddr: String = args.user_vm_bind_sockaddr();

    // Deployed in an L2 VM?
    let in_l2: bool = args.l2();

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

    let procd: LinuxDaemon = match LinuxDaemon::init(
        control_plane_sockaddr,
        control_plane_socket_type,
        user_vm_listener,
        in_l2,
    ) {
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
