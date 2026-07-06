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
#[cfg(target_os = "linux")]
use ::control_plane_api::ControlPlaneRegistrationMessage;
use ::log::{
    debug,
    error,
    info,
};
use ::nanvix_sandbox_config::{
    HostFilter,
    NetworkdEndpoint,
    NetworkingMode,
};
#[cfg(target_os = "linux")]
use ::std::str::FromStr;
use ::std::{
    convert::TryInto,
    env,
    process::ExitCode,
};
#[cfg(target_os = "linux")]
use ::sys::ipc::IkcFrame;
#[cfg(target_os = "linux")]
use ::syscomm::{
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
#[cfg(target_os = "linux")]
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};
#[cfg(target_os = "linux")]
use ::user_vm_api::{
    NEW_USER_VM_MESSAGE_LEN,
    NewUserVm,
    UserVmIdentifier,
};
#[cfg(not(target_os = "linux"))]
use ::uservm::args::UserVmIdentifier;
#[cfg(target_os = "linux")]
use ::uservm::{
    CHANNEL_CAPACITY,
    CONTROL_PLANE_CONNECT_TIMEOUT,
    SYSTEM_VM_CONNECT_TIMEOUT,
    UserVm,
    UserVmArgs,
    counters::MessageCounters,
    io_thread::IoThread,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
    },
};
use ::uservm::{
    args::{
        self,
        Args,
    },
    standalone,
    standalone::StandaloneVmHandle,
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
    // Parse command-line arguments.
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let initrd_args: Option<String> = args.initrd_args();
    let kernel_args: Option<String> = args.kernel_args();
    let ramfs_filename: Option<String> = args.ramfs_filename();
    let stderr: Option<String> = args.take_vm_stderr();
    let user_vm_id: UserVmIdentifier = args.user_vm_id();
    let standalone: bool = args.standalone();
    let snapshot_path: Option<String> = args.take_snapshot_path();
    #[cfg(feature = "gdb")]
    let gdb_port: Option<u16> = args.gdb_port();

    // Initialize logger. If this fails, the program will panic.
    let log_suffix: String = format!("uservm{}", u32::from(user_vm_id));
    ::syslog::init(args.log_to_file(), DEFAULT_LOG_LEVEL, args.log_directory(), Some(log_suffix));

    debug!(
        "main(): starting user VM (user_vm_id={:?}, kernel={:?}, initrd={:?}, ramfs={:?}, \
         standalone={})",
        user_vm_id,
        kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
        standalone
    );

    if standalone {
        let (networking_mode, host_filter, networkd_endpoint): (
            NetworkingMode,
            HostFilter,
            Option<NetworkdEndpoint>,
        ) = standalone_networking(&args)?;
        run_standalone(
            kernel_filename,
            initrd_filename,
            initrd_args,
            kernel_args,
            ramfs_filename,
            stderr,
            snapshot_path,
            networking_mode,
            host_filter,
            networkd_endpoint,
            #[cfg(feature = "gdb")]
            gdb_port,
        )
        .await
    } else {
        #[cfg(target_os = "linux")]
        {
            run_managed(
                args,
                kernel_filename,
                initrd_filename,
                initrd_args,
                kernel_args,
                ramfs_filename,
                stderr,
            )
            .await
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = args;
            error!("main(): managed mode is not supported on this platform, use -standalone");
            Err(anyhow::anyhow!("managed mode is not supported on this platform"))
        }
    }
}

///
/// # Description
///
/// Runs the user VM in standalone mode without connecting to a system VM, control-plane, or
/// gateway. The VM's stdout messages are drained and discarded; the VM's stderr is captured as
/// usual. This mode is useful for debugging and local testing.
///
/// # Parameters
///
/// - `kernel_filename`: Path to the kernel binary.
/// - `initrd_filename`: Optional path to the initrd payload.
/// - `initrd_args`: Optional arguments forwarded to the initrd payload.
/// - `ramfs_filename`: Optional path to a RAM filesystem image.
/// - `stderr`: Optional path to a file used to capture the guest's stderr stream.
/// - `snapshot_path`: Optional path to a snapshot from which to restore VM state instead of
///   cold-booting.
///
/// # Returns
///
/// On success, returns the guest's exit code. On failure, returns an error.
///
/// # Errors
///
/// Returns an error if the VM task panics or if the exit status cannot be converted to a
/// process exit code.
///
#[allow(clippy::too_many_arguments)]
async fn run_standalone(
    kernel_filename: String,
    initrd_filename: Option<String>,
    initrd_args: Option<String>,
    kernel_args: Option<String>,
    ramfs_filename: Option<String>,
    stderr: Option<String>,
    snapshot_path: Option<String>,
    networking_mode: NetworkingMode,
    host_filter: HostFilter,
    networkd_endpoint: Option<NetworkdEndpoint>,
    #[cfg(feature = "gdb")] gdb_port: Option<u16>,
) -> Result<ExitCode> {
    info!(
        "main(): running in standalone mode (no system VM, control-plane, or gateway; \
         networking={networking_mode})"
    );

    let (handle, _io): (StandaloneVmHandle, standalone::StandaloneVmIo) = StandaloneVmHandle::spawn(
        kernel_filename,
        initrd_filename,
        initrd_args,
        kernel_args,
        ramfs_filename,
        stderr,
        snapshot_path,
        None,
        networking_mode,
        host_filter,
        networkd_endpoint,
        #[cfg(feature = "gdb")]
        gdb_port,
    );

    convert_exit_status(handle.wait().await)
}

///
/// # Description
///
/// Builds the standalone networking configuration from parsed command-line arguments.
///
/// Host networking is enabled by `-allow-host-networking`. When a decoupled `networkd` address is
/// supplied (`-networkd-addr`), socket system calls are forwarded to that external process;
/// otherwise the network daemon runs in-process. The host egress filter is always
/// [`HostFilter::AllowAll`] on this path: a decoupled `networkd` enforces its own egress policy,
/// and the in-process daemon applies no filter here.
///
/// # Parameters
///
/// - `args`: Parsed command-line arguments.
///
/// # Returns
///
/// On success, the networking mode, host filter, and optional decoupled `networkd` endpoint.
///
fn standalone_networking(
    args: &Args,
) -> Result<(NetworkingMode, HostFilter, Option<NetworkdEndpoint>)> {
    let networking_mode: NetworkingMode = if args.host_networking_enabled() {
        NetworkingMode::Enabled
    } else {
        NetworkingMode::Disabled
    };
    let networkd_endpoint: Option<NetworkdEndpoint> = build_networkd_endpoint(args)?;
    Ok((networking_mode, HostFilter::AllowAll, networkd_endpoint))
}

///
/// # Description
///
/// Builds the optional decoupled `networkd` endpoint from parsed command-line arguments.
///
/// # Parameters
///
/// - `args`: Parsed command-line arguments.
///
/// # Returns
///
/// On success, the endpoint when `-networkd-addr` was supplied, or `None` for an in-process daemon.
///
#[cfg(target_os = "linux")]
fn build_networkd_endpoint(args: &Args) -> Result<Option<NetworkdEndpoint>> {
    match args.networkd_addr() {
        Some(addr) => {
            let socket_type_str: &str = args.networkd_socket_type().unwrap_or(SocketType::UNIX_STR);
            let socket_type: SocketType = SocketType::from_str(socket_type_str).map_err(|e| {
                anyhow::anyhow!("invalid networkd socket type '{socket_type_str}': {e}")
            })?;
            Ok(Some(NetworkdEndpoint::new(addr.to_string(), socket_type)))
        },
        None => Ok(None),
    }
}

///
/// # Description
///
/// Non-Linux stub: decoupled `networkd` is only supported on Linux, so this always yields `None`.
///
#[cfg(not(target_os = "linux"))]
fn build_networkd_endpoint(args: &Args) -> Result<Option<NetworkdEndpoint>> {
    if args.networkd_addr().is_some() {
        error!("main(): decoupled networkd is not supported on this platform");
    }
    Ok(None)
}

///
/// # Description
///
/// Runs the user VM in managed mode, connecting to the system VM, control-plane, and gateway as
/// required by the full Nanvix deployment.
///
/// # Parameters
///
/// - `args`: Parsed command-line arguments (consumed for socket addresses).
/// - `kernel_filename`: Path to the kernel binary.
/// - `initrd_filename`: Optional path to the initrd payload.
/// - `initrd_args`: Optional arguments forwarded to the initrd payload.
/// - `kernel_args`: Optional kernel arguments written to guest control registers.
/// - `ramfs_filename`: Optional path to a RAM filesystem image.
/// - `stderr`: Optional path to a file used to capture the guest's stderr stream.
///
/// # Returns
///
/// On success, returns the guest's exit code. On failure, returns an error.
///
/// # Errors
///
/// Returns an error if the VM task panics, if socket connections to the system VM or
/// control-plane fail or time out, or if the exit status cannot be converted to a process exit
/// code.
///
#[cfg(target_os = "linux")]
async fn run_managed(
    args: Args,
    kernel_filename: String,
    initrd_filename: Option<String>,
    initrd_args: Option<String>,
    kernel_args: Option<String>,
    ramfs_filename: Option<String>,
    stderr: Option<String>,
) -> Result<ExitCode> {
    // Only the I/O thread channels are required here; the VMM creates its own internally.
    let (vcpu_thread_stdout_tx, io_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    let (io_thread_data_tx, memory_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    let (io_thread_control_tx, io_control_rx) = mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
    let (io_control_tx, io_thread_control_rx) =
        mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

    // Create shared counters for tracking message flow across threads.
    let counters: MessageCounters = MessageCounters::new();

    let unbound_socket: UnboundSocket =
        UnboundSocket::new(SocketType::from_str(args.control_plane_socket_type())?);
    debug!(
        "main(): attempting control plane connection (control_plane_addr={:?}, timeout_ms={})",
        args.control_plane_addr(),
        CONTROL_PLANE_CONNECT_TIMEOUT.as_millis()
    );
    let control_plane_stream: SocketStream = match timeout(
        CONTROL_PLANE_CONNECT_TIMEOUT,
        unbound_socket.connect(args.control_plane_addr()),
    )
    .await
    {
        Ok(Ok(mut stream)) => {
            let registration: Vec<u8> =
                ControlPlaneRegistrationMessage::for_uservm(args.user_vm_id()).to_bytes()?;
            stream.write_all(&registration).await?;
            info!(
                "main(): connected to control plane (control_plane_addr={:?})",
                args.control_plane_addr()
            );
            stream
        },
        Ok(Err(e)) => {
            let reason: String = format!(
                "failed to connect to control plane (control_plane_addr={:?}, error={e:?})",
                args.control_plane_addr()
            );
            error!("main(): {reason}");
            return Err(anyhow::anyhow!("{reason}"));
        },
        Err(_) => {
            let reason: String = format!(
                "timed out trying to connect to control plane (control_plane_addr={:?})",
                args.control_plane_addr()
            );
            error!("main(): {reason}");
            return Err(anyhow::anyhow!("{reason}"));
        },
    };

    // Connect to the system VM.
    let unbound_socket: UnboundSocket =
        UnboundSocket::new(SocketType::from_str(args.system_vm_socket_type())?);
    debug!(
        "main(): attempting system VM connection (system_vm_addr={:?}, timeout_ms={})",
        args.system_vm_addr(),
        SYSTEM_VM_CONNECT_TIMEOUT.as_millis()
    );
    let system_vm_stream: SocketStream =
        match timeout(SYSTEM_VM_CONNECT_TIMEOUT, unbound_socket.connect(args.system_vm_addr()))
            .await
        {
            Ok(Ok(mut stream)) => {
                info!(
                    "main(): connected to system VM (system_vm_addr={:?})",
                    args.system_vm_addr()
                );
                let new_msg: NewUserVm = match NewUserVm::new(
                    args.user_vm_id(),
                    args.gateway_addr().to_string(),
                    SocketType::from_str(args.gateway_socket_type())?,
                ) {
                    Ok(message) => message,
                    Err(e) => {
                        let reason: String = format!(
                            "failed to construct user VM registration message (error={e:?})"
                        );
                        error!("main(): {reason}");
                        return Err(anyhow::anyhow!(reason));
                    },
                };

                debug!(
                    "main(): registering gateway with system VM (gateway_addr={:?}, \
                     gateway_socket_type={:?})",
                    args.gateway_addr(),
                    args.gateway_socket_type()
                );
                debug!("forwarding user vm information to system vm");
                let new_msg_bytes: [u8; NEW_USER_VM_MESSAGE_LEN] = new_msg.to_bytes();
                if let Err(e) = stream.write_all(&new_msg_bytes).await {
                    let reason: String =
                        format!("failed to send user VM registration message (error={e:?})");
                    error!("main(): {reason}");
                    return Err(anyhow::anyhow!(reason));
                }
                stream
            },
            Ok(Err(e)) => {
                let reason: String = format!(
                    "failed to connect to system VM (system_vm_addr={:?}, error={e:?})",
                    args.system_vm_addr()
                );
                error!("main(): {reason}");
                return Err(anyhow::anyhow!("{reason}"));
            },
            Err(_) => {
                let reason: String = format!(
                    "timed out trying to connect to system VM (system_vm_addr={:?})",
                    args.system_vm_addr()
                );
                error!("main(): {reason}");
                return Err(anyhow::anyhow!("{reason}"));
            },
        };

    // Spawn I/O thread.
    let io_thread: JoinHandle<Result<()>> = IoThread::spawn(
        system_vm_stream,
        io_thread_data_rx,
        io_thread_data_tx,
        io_thread_control_tx,
        io_thread_control_rx,
        control_plane_stream,
        counters.clone(),
    )?;
    debug!("main(): spawned I/O thread (channel_capacity={})", CHANNEL_CAPACITY);

    // Run virtual machine and check exit status code.
    debug!(
        "main(): launching uservm (kernel={:?}, initrd={:?}, ramfs={:?})",
        kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
    );
    let vmm_handle: JoinHandle<Result<u16>> = UserVm::spawn(UserVmArgs {
        initrd_filename,
        initrd_args,
        kernel_args,
        ramfs_filename,
        stderr,
        vcpu_thread_stdout_tx,
        memory_thread_data_rx,
        io_control_rx,
        io_control_tx,
        kernel_filename,
        counters,
        snapshot_path: None,
        #[cfg(feature = "gdb")]
        gdb_port: None,
        #[cfg(feature = "profile-time")]
        perf_timings: ::uservm::perf::PerfTimings::new(),
        guest_profile_path: std::env::var("NANVIX_GUEST_PROFILE_PATH").ok(),
    });

    let vm_exit_status: Result<u16> = vmm_handle.await?;
    debug!("main(): uservm completed (exit_status={vm_exit_status:?})");

    if let Err(error) = io_thread.await? {
        // Don't bail as we want to return the VM's exit code.
        let reason: String = format!("I/O thread failed (error={error:?})");
        error!("main(): {reason}");
    }

    convert_exit_status(vm_exit_status)
}

///
/// # Description
///
/// Converts a VM exit status into a process exit code.
///
/// # Parameters
///
/// - `vm_exit_status`: The exit status returned by the virtual machine.
///
/// # Returns
///
/// On success, returns the corresponding [`ExitCode`]. On failure, returns an error.
///
/// # Errors
///
/// Returns an error if the VM itself failed or if the exit status exceeds the range of `u8`.
///
fn convert_exit_status(vm_exit_status: Result<u16>) -> Result<ExitCode> {
    match vm_exit_status {
        Ok(0) => Ok(ExitCode::from(0)),
        Ok(exit_status) if exit_status != 0 => {
            let exit_code_result: ::std::result::Result<u8, ::std::num::TryFromIntError> =
                exit_status.try_into();
            match exit_code_result {
                Ok(code) => Ok(ExitCode::from(code)),
                Err(_) => {
                    let reason: String =
                        format!("failed to convert exit status (exit_status={exit_status})");
                    error!("main(): {reason}");
                    Err(anyhow::anyhow!(reason))
                },
            }
        },
        _ => {
            let reason: String = "virtual machine failed".to_string();
            error!("main(): {reason}");
            Err(anyhow::anyhow!(reason))
        },
    }
}
