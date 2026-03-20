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
    warn,
};
#[cfg(all(feature = "microvm", feature = "ring-buffer"))]
use ::std::{fs::OpenOptions, path::PathBuf};
use ::std::{
    convert::TryInto,
    env,
    process::ExitCode,
    str::FromStr,
};
use ::sys::ipc::IkcFrame;
use ::syscomm::{
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};
use ::user_vm_api::{
    NEW_USER_VM_MESSAGE_LEN,
    NewUserVm,
    RingTransport,
    UserVmIdentifier,
};
use ::uservm::{
    CHANNEL_CAPACITY,
    CONTROL_PLANE_CONNECT_TIMEOUT,
    SYSTEM_VM_CONNECT_TIMEOUT,
    UserVm,
    UserVmArgs,
    args::{
        self,
        Args,
    },
    counters::MessageCounters,
    io_thread::IoThread,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
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
    // Parse command-line arguments.
    let mut args: Args = args::Args::parse(env::args().collect())?;
    let kernel_filename: String = args.kernel_filename().to_string();
    let initrd_filename: Option<String> = args.initrd_filename();
    let initrd_args: Option<String> = args.initrd_args();
    let ramfs_filename: Option<String> = args.ramfs_filename();
    let memory_size: usize = args.memory_size();
    let stderr: Option<String> = args.take_vm_stderr();
    let user_vm_id: UserVmIdentifier = args.user_vm_id();
    let standalone: bool = args.standalone();
    let snapshot_path: Option<String> = args.take_snapshot_path();
    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
    let ring_shared_path_from_launcher: Option<String> = args.take_ring_shared_path();
    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
    let disable_ring_buffer: bool = args.disable_ring_buffer();

    // Initialize logger. If this fails, the program will panic.
    ::syslog::init(
        args.log_to_file(),
        DEFAULT_LOG_LEVEL,
        args.log_directory(),
        Some(format!("uservm{}", u32::from(user_vm_id))),
    );

    debug!(
        "main(): starting user VM (user_vm_id={:?}, kernel={:?}, initrd={:?}, ramfs={:?}, \
         memory_size_bytes={}, standalone={})",
        user_vm_id,
        &kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
        memory_size,
        standalone
    );

    if standalone {
        run_standalone(
            kernel_filename,
            initrd_filename,
            initrd_args,
            ramfs_filename,
            memory_size,
            stderr,
            snapshot_path,
        )
        .await
    } else {
        run_managed(
            args,
            kernel_filename,
            initrd_filename,
            initrd_args,
            ramfs_filename,
            memory_size,
            stderr,
            #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
            ring_shared_path_from_launcher,
            #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
            disable_ring_buffer,
        )
        .await
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
/// - `memory_size`: Amount of guest physical memory in bytes.
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
async fn run_standalone(
    kernel_filename: String,
    initrd_filename: Option<String>,
    initrd_args: Option<String>,
    ramfs_filename: Option<String>,
    memory_size: usize,
    stderr: Option<String>,
    snapshot_path: Option<String>,
) -> Result<ExitCode> {
    info!("main(): running in standalone mode (no system VM, control-plane, or gateway)");

    // Create channels. In standalone mode these are wired directly without an I/O thread.
    let (vcpu_thread_stdout_tx, mut standalone_data_rx) =
        mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    // Nobody sends inbound data in standalone mode. The sender is kept alive so that the memory
    // thread's receiver does not see an immediate channel close.
    let (_inbound_data_tx, memory_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    // Kept alive so the orchestrator's io_control_rx does not see an immediate channel close.
    let (_io_cmd_tx, io_control_rx) = mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
    // Kept alive so the orchestrator can send control responses without a closed-channel error.
    let (io_control_tx, _io_resp_rx) = mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

    let counters: MessageCounters = MessageCounters::new();

    let vmm_handle: JoinHandle<Result<u16>> = UserVm::spawn(UserVmArgs {
        memory_size,
        initrd_filename,
        initrd_args,
        ramfs_filename,
        stderr,
        vcpu_thread_stdout_tx,
        memory_thread_data_rx,
        io_control_rx,
        io_control_tx,
        kernel_filename,
        counters,
        snapshot_path,
        #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
        ring_shared_path: None,
    });

    // Drain the VM's stdout channel. In standalone mode there is no system VM to forward messages
    // to, so we simply consume and discard them to prevent the channel from blocking the VM.
    let drain_handle: JoinHandle<()> = tokio::spawn(async move {
        while let Some(_msg) = standalone_data_rx.recv().await {}
        debug!("main(): standalone mode: VM stdout channel closed");
    });

    let vm_exit_status: Result<u16> = vmm_handle.await?;
    debug!("main(): uservm completed (exit_status={vm_exit_status:?})");

    // Wait for the drain task to finish.
    if let Err(error) = drain_handle.await {
        warn!("main(): standalone drain task failed (error={error:?})");
    }

    convert_exit_status(vm_exit_status)
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
/// - `ramfs_filename`: Optional path to a RAM filesystem image.
/// - `memory_size`: Amount of guest physical memory in bytes.
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
async fn run_managed(
    args: Args,
    kernel_filename: String,
    initrd_filename: Option<String>,
    initrd_args: Option<String>,
    ramfs_filename: Option<String>,
    memory_size: usize,
    stderr: Option<String>,
    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
    ring_shared_path_from_launcher: Option<String>,
    #[cfg(all(feature = "microvm", feature = "ring-buffer"))] disable_ring_buffer: bool,
) -> Result<ExitCode> {
    // Only the I/O thread channels are required here; the VMM creates its own internally.
    let (vcpu_thread_stdout_tx, io_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    let (io_thread_data_tx, memory_thread_data_rx) = mpsc::channel::<IkcFrame>(CHANNEL_CAPACITY);
    let (io_thread_control_tx, io_control_rx) = mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
    let (io_control_tx, io_thread_control_rx) =
        mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

    // Create shared counters for tracking message flow across threads.
    let counters: MessageCounters = MessageCounters::new();

    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
    struct RingSharedBacking {
        path: PathBuf,
        owned_by_uservm: bool,
    }

    let ring_shared_backing: Option<RingSharedBacking> = if disable_ring_buffer {
        None
    } else if let Some(path) = ring_shared_path_from_launcher {
        Some(RingSharedBacking {
            path: PathBuf::from(path),
            owned_by_uservm: false,
        })
    } else {
        Some(RingSharedBacking {
            path: prepare_shared_ring_backing(args.user_vm_id())?,
            owned_by_uservm: true,
        })
    };

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
        Ok(Ok(stream)) => {
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
                    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
                    match ring_shared_backing.as_ref() {
                        Some(backing) => {
                            RingTransport::file_path(backing.path.display().to_string())?
                        },
                        None => RingTransport::disabled(),
                    },
                    #[cfg(not(all(feature = "microvm", feature = "ring-buffer")))]
                    RingTransport::disabled(),
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
        "main(): launching uservm (kernel={:?}, initrd={:?}, ramfs={:?}, memory_size_bytes={})",
        &kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
        memory_size
    );
    let vmm_handle: JoinHandle<Result<u16>> = UserVm::spawn(UserVmArgs {
        memory_size,
        initrd_filename,
        initrd_args,
        ramfs_filename,
        stderr,
        vcpu_thread_stdout_tx,
        memory_thread_data_rx,
        io_control_rx,
        io_control_tx,
        kernel_filename,
        counters,
        snapshot_path: None,
        #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
        ring_shared_path: ring_shared_backing
            .as_ref()
            .map(|backing| backing.path.display().to_string()),
    });

    let vm_exit_status: Result<u16> = vmm_handle.await?;
    debug!("main(): uservm completed (exit_status={vm_exit_status:?})");

    if let Err(error) = io_thread.await? {
        // Don't bail as we want to return the VM's exit code.
        let reason: String = format!("I/O thread failed (error={error:?})");
        error!("main(): {reason}");
    }

    #[cfg(all(feature = "microvm", feature = "ring-buffer"))]
    if let Some(ring_shared_backing) = ring_shared_backing.as_ref() {
        if ring_shared_backing.owned_by_uservm {
            if let Err(error) = std::fs::remove_file(&ring_shared_backing.path) {
                warn!(
                    "main(): failed to remove shared ring backing file (path={}, error={error:?})",
                    ring_shared_backing.path.display()
                );
            }
        }
    }

    convert_exit_status(vm_exit_status)
}

#[cfg(all(feature = "microvm", feature = "ring-buffer"))]
fn prepare_shared_ring_backing(user_vm_id: UserVmIdentifier) -> Result<PathBuf> {
    let base_dir: PathBuf = std::env::temp_dir();
    std::fs::create_dir_all(&base_dir).map_err(|e| {
        anyhow::anyhow!("failed to create shared ring backing directory {:?}: {e}", base_dir)
    })?;
    let path: PathBuf = base_dir.join(format!("nanvix-ring-{}.shm", u32::from(user_vm_id)));

    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| anyhow::anyhow!("failed to create shared ring backing file {:?}: {e}", path))?;

    file.set_len(::config::microvm::RING_BUFFER_SIZE as u64).map_err(|e| {
        anyhow::anyhow!(
            "failed to size shared ring backing file {:?} to {} bytes: {e}",
            path,
            ::config::microvm::RING_BUFFER_SIZE
        )
    })?;

    Ok(path)
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
