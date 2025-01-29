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
    os::unix::net::UnixStream,
    time::Duration,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<()> {
    // Parse command-line arguments.
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let memory_size: usize = args.memory_size();
    let stderr: Option<String> = args.take_vm_stderr();
    let gateway_addr: Option<String> = args.gateway_addr();

    // Initialize logger. If this fails, the program will panic.
    logging::initialize(args.log_to_file());

    let gateway: Option<Gateway> = match &gateway_addr {
        Some(addr) => match UnixStream::connect(addr.clone()) {
            Ok(conn) => {
                conn.set_read_timeout(Some(Duration::from_millis(1)))?;
                Some(Gateway::UnixStream(conn))
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

    let mut vmm: Vmm = Vmm::new(memory_size, &kernel_filename, initrd_filename, stderr, gateway)?;

    vmm.run()?;

    Ok(())
}
