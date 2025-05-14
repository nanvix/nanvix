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
    env,
    str::FromStr,
};
use ::syscomm::{
    SocketStream,
    SocketType,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default socket bind type.
const DEFAULT_BIND_SOCKET_TYPE: SocketType = SocketType::Unix;

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<()> {
    // Parse command-line arguments.
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let initrd_args: Option<String> = args.initrd_args();
    let memory_size: usize = args.memory_size();
    let stderr: Option<String> = args.take_vm_stderr();
    let gateway_addr: Option<String> = args.gateway_addr();

    let gateway_socket_type: SocketType = match args.gateway_socket_type() {
        Some(typ) => match SocketType::from_str(typ.as_str()) {
            Ok(typ) => typ,
            Err(error) => {
                error!("{error} (type={:?})", typ);
                anyhow::bail!("failed to parse socket address type");
            },
        },
        None => DEFAULT_BIND_SOCKET_TYPE,
    };

    // Initialize logger. If this fails, the program will panic.
    logging::initialize(args.log_to_file());

    let gateway: Option<Gateway> = match &gateway_addr {
        Some(addr) => match SocketStream::connect(gateway_socket_type, addr.clone()) {
            Ok(stream) => {
                stream.set_nonblocking(true)?;
                Some(Gateway::new(stream))
            },
            Err(e) => {
                let reason: String = format!(
                    "failed to connect to gateway (gateway_addr={:?}, error={:?})",
                    addr, e
                );
                error!("main()(): {}", reason);
                anyhow::bail!(reason)
            },
        },
        None => None,
    };

    let mut vmm: Vmm =
        Vmm::new(memory_size, &kernel_filename, initrd_filename, initrd_args, stderr, gateway)?;

    // Run virtual machine and check exit status code.
    match vmm.run()? {
        exit_status if exit_status != 0 => {
            error!("main(): virtual machine exited with status {}", (exit_status as i16));
            Err(anyhow::anyhow!("virtual machine exited with status {}", exit_status))
        },
        _ => Ok(()),
    }
}
