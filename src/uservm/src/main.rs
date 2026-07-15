// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

use ::anyhow::Result;
use ::log::{
    debug,
    error,
    info,
};
use ::nanvix_sandbox_config::{
    HostFilter,
    NetworkingMode,
};
use ::std::{
    env,
    process::ExitCode,
};
use ::uservm::{
    args::{
        self,
        Args,
    },
    standalone::{
        StandaloneVmHandle,
        StandaloneVmIo,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[tokio::main]
pub async fn main() -> Result<ExitCode> {
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let initrd_args: Option<String> = args.initrd_args();
    let kernel_args: Option<String> = args.kernel_args();
    let ramfs_filename: Option<String> = args.ramfs_filename();
    let stderr: Option<String> = args.take_vm_stderr();
    let snapshot_path: Option<String> = args.take_snapshot_path();
    #[cfg(feature = "gdb")]
    let gdb_port: Option<u16> = args.gdb_port();

    let log_suffix: String = format!("uservm{}", u32::from(args.user_vm_id()));
    ::syslog::init(args.log_to_file(), DEFAULT_LOG_LEVEL, args.log_directory(), Some(log_suffix));

    debug!(
        "main(): starting user VM (user_vm_id={}, kernel={:?}, initrd={:?}, ramfs={:?})",
        args.user_vm_id(),
        kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
    );

    run(
        kernel_filename,
        initrd_filename,
        initrd_args,
        kernel_args,
        ramfs_filename,
        stderr,
        snapshot_path,
        #[cfg(feature = "gdb")]
        gdb_port,
    )
    .await
}

///
/// # Description
///
/// Runs a user VM without external system VM, control-plane, or gateway connections.
///
/// # Returns
///
/// On success, returns the guest's exit code. On failure, returns an error.
///
#[allow(clippy::too_many_arguments)]
async fn run(
    kernel_filename: String,
    initrd_filename: Option<String>,
    initrd_args: Option<String>,
    kernel_args: Option<String>,
    ramfs_filename: Option<String>,
    stderr: Option<String>,
    snapshot_path: Option<String>,
    #[cfg(feature = "gdb")] gdb_port: Option<u16>,
) -> Result<ExitCode> {
    info!("main(): running user VM");

    let (handle, _io): (StandaloneVmHandle, StandaloneVmIo) = StandaloneVmHandle::spawn(
        kernel_filename,
        initrd_filename,
        initrd_args,
        kernel_args,
        ramfs_filename,
        stderr,
        snapshot_path,
        None,
        NetworkingMode::Disabled,
        HostFilter::AllowAll,
        #[cfg(feature = "gdb")]
        gdb_port,
    );

    convert_exit_status(handle.wait().await)
}

///
/// # Description
///
/// Converts a VM exit status into a process exit code.
///
/// # Parameters
///
/// - `vm_exit_status`: Exit status returned by the virtual machine.
///
/// # Returns
///
/// On success, returns the corresponding [`ExitCode`]. On failure, returns an error.
///
fn convert_exit_status(vm_exit_status: Result<u16>) -> Result<ExitCode> {
    match vm_exit_status {
        Ok(0) => Ok(ExitCode::from(0)),
        Ok(exit_status) => match u8::try_from(exit_status) {
            Ok(code) => Ok(ExitCode::from(code)),
            Err(error) => {
                let reason: String =
                    format!("failed to convert exit status (exit_status={exit_status}): {error}");
                error!("main(): {reason}");
                Err(::anyhow::anyhow!(reason))
            },
        },
        Err(error) => {
            let reason: String = format!("virtual machine failed: {error}");
            error!("main(): {reason}");
            Err(::anyhow::anyhow!(reason))
        },
    }
}
