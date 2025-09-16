// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # MicroVM
//!
//! MicroVM is a ultra-lightweight virtual machine that is designed to run the
//! [Nanvix](https://github.com/nanvix/) operating system. Currently Linux KVM is supported as
//! backend.
//!

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod logging;

//==================================================================================================
// Imports
//==================================================================================================

/// Must come first.
#[macro_use]
extern crate log;

use self::args::Args;
use ::anyhow::Result;
use ::microvm::{
    Gateway,
    Vmm,
};
use ::std::{
    convert::TryInto,
    env,
    process::ExitCode,
    str::FromStr,
    time::Duration,
};
use ::syscomm::{
    BlockingSocketStream,
    SocketStream,
    SocketType,
};
use ::user_vm_api::NewUserVm;

//==================================================================================================
// Constants
//==================================================================================================

/// Default socket bind type.
const DEFAULT_SYSTEM_VM_SOCKET_TYPE: SocketType = SocketType::Unix;
const DEFAULT_CONTROL_PLANE_SOCKET_TYPE: SocketType = SocketType::Unix;
const DEFAULT_GATEWAY_SOCKET_TYPE: SocketType = SocketType::Unix;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<ExitCode> {
    // Parse command-line arguments.
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let initrd_args: Option<String> = args.initrd_args();
    let memory_size: usize = args.memory_size();
    let stderr: Option<String> = args.take_vm_stderr();
    let system_vm_addr: Option<String> = args.system_vm_addr();

    let system_vm_socket_type: SocketType = match args.system_vm_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={typ:?})");
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_SYSTEM_VM_SOCKET_TYPE,
    };

    // Initialize logger. If this fails, the program will panic.
    logging::initialize(args.log_to_file());

    let gateway: Option<Gateway> = match &system_vm_addr {
        Some(addr) => {
            match SocketStream::connect_timeout(
                system_vm_socket_type,
                addr.clone(),
                Duration::from_secs(config::syscomm::CONNECT_TIMEOUT_SECS),
            ) {
                Ok(stream) => {
                    let gateway_socket_type: SocketType = match args.gateway_socket_type() {
                        Some(socket_type) => socket_type,
                        None => DEFAULT_GATEWAY_SOCKET_TYPE,
                    };
                    if let Some(gateway_sockaddr) = args.gateway_addr() {
                        let mut blocking_stream: BlockingSocketStream = stream.set_blocking()?;
                        let new_msg: NewUserVm = NewUserVm::new(
                            args.user_vm_id(),
                            gateway_sockaddr,
                            gateway_socket_type,
                        );
                        new_msg.send(&mut blocking_stream)?;

                        Some(Gateway::new(blocking_stream.set_nonblocking()?))
                    } else {
                        let reason: String = "configured user VM with system VM but without \
                                              gateway address"
                            .to_string();
                        error!("main() {reason}");
                        anyhow::bail!(reason);
                    }
                },
                Err(e) => {
                    let reason: String = format!(
                        "failed to connect to system VM (system_vm_addr={addr:?}, error={e:?})",
                    );
                    error!("main(): {reason}");
                    anyhow::bail!(reason)
                },
            }
        },
        None => None,
    };

    let _control_plane_socket: Option<SocketStream> = match args.control_plane_addr() {
        Some(addr) => {
            let control_plane_socket_type: SocketType = match args.control_plane_socket_type() {
                Some(socket_type) => socket_type,
                None => DEFAULT_CONTROL_PLANE_SOCKET_TYPE,
            };
            match SocketStream::connect(control_plane_socket_type, addr.clone()) {
                Ok(stream) => Some(stream),
                Err(e) => {
                    let reason: String = format!(
                        "failed to connect to control-plane (control_plane_addr={addr:?}, \
                         error={e:?})",
                    );
                    error!("main(): {reason}");
                    return Err(anyhow::anyhow!("{reason}"));
                },
            }
        },
        None => None,
    };

    // Run virtual machine and check exit status code.
    match Vmm::spawn(memory_size, &kernel_filename, initrd_filename, initrd_args, stderr, gateway)?
    {
        exit_status if exit_status != 0 => {
            let exit_code: u8 = match exit_status.try_into() {
                Ok(code) => code,
                Err(_) => {
                    let reason: &str = "failed to convert exit status";
                    error!("main(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            Ok(ExitCode::from(exit_code))
        },
        _ => Ok(ExitCode::from(0)),
    }
}
