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
use ::std::{
    convert::TryInto,
    env,
    process::ExitCode,
    str::FromStr,
};
use ::sys::ipc::Message;
use ::syscomm::{
    SocketStream,
    SocketType,
    UnboundSocket,
    WriteAll,
};
use ::syslog::{
    debug,
    error,
    info,
};
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::timeout,
};
use ::user_vm_api::{
    NEW_USER_VM_MESSAGE_LEN,
    NewUserVm,
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

    // Initialize logger. If this fails, the program will panic.
    ::syslog::init(
        args.log_to_file(),
        DEFAULT_LOG_LEVEL,
        args.log_directory(),
        Some(format!("uservm{}", u32::from(user_vm_id))),
    );

    debug!(
        "main(): starting user VM (user_vm_id={:?}, kernel={:?}, initrd={:?}, ramfs={:?}, \
         memory_size_bytes={})",
        user_vm_id,
        &kernel_filename,
        initrd_filename.as_deref().unwrap_or("none"),
        ramfs_filename.as_deref().unwrap_or("none"),
        memory_size
    );

    // Only the I/O thread channels are required here; the VMM creates its own internally.
    let (vcpu_thread_stdout_tx, io_thread_data_rx) = mpsc::channel::<Message>(CHANNEL_CAPACITY);
    let (io_thread_data_tx, memory_thread_data_rx) = mpsc::channel::<Message>(CHANNEL_CAPACITY);
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
    });

    let vm_exit_status: Result<u16> = vmm_handle.await?;
    debug!("main(): uservm completed (exit_status={vm_exit_status:?})");

    if let Err(error) = io_thread.await? {
        // Don't bail as we want to return the VM's exit code.
        let reason: String = format!("I/O thread failed (error={error:?})");
        error!("main(): {reason}");
    }

    let result: Result<ExitCode> = match vm_exit_status {
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
    };

    result
}
