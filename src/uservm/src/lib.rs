// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! High-level orchestration layer for user virtual machines.
//!
//! This crate wires together the asynchronous runtime, virtual machine monitor (VMM), and
//! supporting worker threads that collectively emulate a user virtual machine. It exposes a public
//! API consumed by the Linux daemon and control-plane components to spawn, supervise, and interact
//! with UserVM instances.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// Lints
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
#![forbid(clippy::cast_possible_truncation)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod args;
/// Library module for manipulating ELF binaries.
pub mod elf;
pub mod io_thread;
pub mod memory_thread;
pub mod orchestrator;
pub mod vmm;

pub mod pal;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory_thread::{
        AddCreditFn,
        MemoryThread,
    },
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
        LoadSnapshotFn,
        MemoryControlCommand,
        MemoryControlResponse,
        Orchestrator,
        PauseFn,
        ResumeFn,
        VcpuControlCommand,
        VcpuControlResponse,
    },
    vmm::{
        MicroVmArgs,
        StdinFn,
        StdoutFn,
        VirtualMemory,
        Vmm,
        guest::Guest,
    },
};
use ::anyhow::Result;
use ::config::syscomm::DEFAULT_CHANNEL_CAPACITY;
use ::std::{
    fs::File,
    io::Write,
    sync::Arc,
};
use ::sys::ipc::{
    Message,
    MessageType,
};
use ::syslog::{
    error,
    trace,
};
use ::tokio::{
    sync::{
        Mutex,
        MutexGuard,
        mpsc,
        mpsc::{
            Receiver,
            Sender,
        },
    },
    task::JoinHandle,
};

//==================================================================================================
// VmmArgs
//==================================================================================================

/// Bundles all resources required to spawn a [`UserVm`] instance.
pub struct UserVmArgs {
    /// Amount of guest physical memory to allocate for the virtual machine, in bytes.
    pub memory_size: usize,
    /// Absolute or relative path to the guest kernel image to boot.
    pub kernel_filename: String,
    /// Optional path to an initrd payload that should be exposed to the guest.
    pub initrd_filename: Option<String>,
    /// Optional string of arguments forwarded to the initrd payload.
    pub initrd_args: Option<String>,
    /// Optional path to a file used to capture the guest's stderr stream.
    pub stderr: Option<String>,
    /// Channel used to forward port-I/O writes from the guest to the Linux daemon.
    pub vcpu_thread_stdout_tx: Sender<Message>,
    /// Channel providing messages emitted by the Linux daemon destined for the guest.
    pub memory_thread_data_rx: Receiver<Message>,
    /// Channel receiving control commands from the orchestrator's I/O subsystem.
    pub io_control_rx: Receiver<IoControlCommand>,
    /// Channel transmitting responses back to the orchestrator's I/O subsystem.
    pub io_control_tx: Sender<IoControlResponse>,
}

//==================================================================================================
// VMM
//==================================================================================================

/// Asynchronous facade responsible for instantiating and supervising a user virtual machine.
pub struct UserVm;

impl UserVm {
    /// Launches a user virtual machine on a Tokio task and returns a handle to its completion.
    pub fn spawn(args: UserVmArgs) -> JoinHandle<Result<u16>> {
        tokio::spawn(async move { UserVm::run(args).await })
    }

    /// Runs the asynchronous event loop powering a user virtual machine.
    ///
    /// # Description
    ///
    /// Instantiates and runs the virtual machine monitor (VMM) using the provided arguments,
    /// wiring up the orchestrator, memory thread, and I/O plumbing required to communicate with
    /// the system VM.
    ///
    /// # Parameters
    ///
    /// - `args`: Aggregation of configuration values and channels consumed by the VMM.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    async fn run(args: UserVmArgs) -> Result<u16> {
        trace!("spawn()");
        let (memory_thread_data_tx, vcpu_thread_stdin_rx): (Sender<Message>, Receiver<Message>) =
            mpsc::channel::<Message>(DEFAULT_CHANNEL_CAPACITY);
        let (memory_control_tx, memory_thread_control_rx): (
            Sender<MemoryControlCommand>,
            Receiver<MemoryControlCommand>,
        ) = mpsc::channel::<MemoryControlCommand>(DEFAULT_CHANNEL_CAPACITY);
        let (memory_thread_control_tx, memory_control_rx): (
            Sender<MemoryControlResponse>,
            Receiver<MemoryControlResponse>,
        ) = mpsc::channel::<MemoryControlResponse>(DEFAULT_CHANNEL_CAPACITY);
        let (vcpu_control_tx, vcpu_thread_control_rx): (
            Sender<VcpuControlCommand>,
            Receiver<VcpuControlCommand>,
        ) = mpsc::channel::<VcpuControlCommand>(DEFAULT_CHANNEL_CAPACITY);
        let (vcpu_thread_control_tx, mut vcpu_control_rx): (
            Sender<VcpuControlResponse>,
            Receiver<VcpuControlResponse>,
        ) = mpsc::channel::<VcpuControlResponse>(DEFAULT_CHANNEL_CAPACITY);

        let vmm_stderr_fn: Box<dyn Write + Send> = get_stderr_writer(args.stderr.clone())?;

        // Output function used for emulating I/O port writes.
        let vmm_stdout_fn: Box<StdoutFn> = output_fn(args.vcpu_thread_stdout_tx.clone());

        // Input function used for emulating I/O port reads.
        let vmm_stdin_fn: Box<StdinFn> = build_input_fn(vcpu_thread_stdin_rx);

        let microvm: Vmm = Vmm::new(MicroVmArgs {
            input: vmm_stdin_fn,
            output: vmm_stdout_fn,
            stderr: vmm_stderr_fn,
            memory_size: args.memory_size,
            control_rx: vcpu_thread_control_rx,
            control_tx: vcpu_thread_control_tx,
            kernel_filename: args.kernel_filename,
            initrd_filename: args.initrd_filename.clone(),
            initrd_args: args.initrd_args.clone(),
        })?;

        let vmem: Arc<Mutex<VirtualMemory>> = microvm.vmem();
        let guest: Arc<Mutex<Guest>> = microvm.guest();

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread: MemoryThread = MemoryThread::new(
            args.memory_thread_data_rx,
            memory_thread_data_tx,
            memory_thread_control_rx,
            memory_thread_control_tx,
            add_credit_fn(guest.clone(), vmem.clone()),
        );
        let memory_thread: JoinHandle<()> = memory_thread.spawn();

        let vmm_thread: JoinHandle<Result<u16>> = Vmm::spawn(microvm.clone());

        // Wait right after spawning the vCPU thread such that we populate the pthread id holder
        // before actually starting the vCPU.
        let vcpu_tid: u64 = match vcpu_control_rx.recv().await {
            Some(VcpuControlResponse::Tid(tid)) => {
                trace!("Received vCPU thread tid: {tid}");
                tid
            },
            _ => {
                let reason: String = "the vCPU thread has disconnected".to_string();
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let filename: String = args
            .initrd_filename
            .unwrap_or("bin/default.elf".to_string());
        let orchestrator_thread: Orchestrator = Orchestrator::new(
            vcpu_tid,
            args.io_control_rx,
            args.io_control_tx,
            memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            pause_microvm(guest.clone(), vmem.clone()),
            resume_microvm(guest.clone(), vmem.clone()),
            create_snapshot_fn(microvm.clone(), filename.clone()),
            load_snapshot_fn(microvm.clone(), filename.clone()),
        );

        let orchestrator_thread_handle: JoinHandle<Result<()>> = orchestrator_thread.spawn();

        let exit_code: Result<u16> = match vmm_thread.await {
            Ok(exit_code) => exit_code,
            Err(error) => {
                let reason: String = format!("failed to join vmm thread (error={error:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        };

        if let Err(error) = orchestrator_thread_handle.await {
            error!("spawn(): failed to join orchestrator thread (error={error:?})");
            // Don't bail, in order to cleanup the other the other tasks properly.
        }

        if let Err(error) = memory_thread.await {
            error!("spawn(): error joining memory thread (error={error:?})");
            // Don't bail, in order to cleanup the other the other tasks properly.
        }

        exit_code
    }
}

fn create_snapshot_fn(microvm: Vmm, filename: String) -> Box<LoadSnapshotFn> {
    Box::new(move || {
        let microvm = microvm.clone();
        let filename = filename.clone();
        Box::pin(async move { microvm.create_snapshot(filename).await })
    })
}

fn load_snapshot_fn(microvm: Vmm, filename: String) -> Box<LoadSnapshotFn> {
    Box::new(move || {
        let microvm = microvm.clone();
        let filename = filename.clone();
        Box::pin(async move { microvm.load_snapshot(filename).await })
    })
}

fn pause_microvm(guest: Arc<Mutex<Guest>>, vmem: Arc<Mutex<VirtualMemory>>) -> Box<PauseFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        Box::pin(async move {
            let mut guest = guest.lock().await;
            let mut vmem: MutexGuard<VirtualMemory> = vmem.lock().await;
            guest.pause_vm(&mut vmem)
        })
    })
}

fn resume_microvm(guest: Arc<Mutex<Guest>>, vmem: Arc<Mutex<VirtualMemory>>) -> Box<ResumeFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        Box::pin(async move {
            let mut guest = guest.lock().await;
            let mut vmem: MutexGuard<VirtualMemory> = vmem.lock().await;
            guest.resume_vm(&mut vmem)
        })
    })
}

fn add_credit_fn(guest: Arc<Mutex<Guest>>, vmem: Arc<Mutex<VirtualMemory>>) -> Box<AddCreditFn> {
    Box::new(move || {
        let guest: Arc<Mutex<Guest>> = guest.clone();
        let vmem: Arc<Mutex<VirtualMemory>> = vmem.clone();
        Box::pin(async move {
            let mut guest = guest.lock().await;
            let mut vmem = vmem.lock().await;
            guest.add_credit(&mut vmem)
        })
    })
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Obtains a buffered writer for the virtual machine's standard error device.
///
/// When a file path is provided the writer targets the specified file, creating or truncating it
/// as needed. Otherwise the host process' standard error stream is used.
///
/// # Parameters
///
/// * `vm_stderr` - Optional file path used to capture the guest's stderr output.
///
/// # Returns
///
/// On success, the function returns a buffered writer for the virtual machine's standard error
/// stream. An error is returned when the target file cannot be created.
///
pub fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write + Send>> {
    // Obtain a buffered writer for the virtual machine's standard error device.
    let file_writer: Box<dyn Write + Send> = if let Some(vm_stderr) = vm_stderr {
        // Standard error was set to a file. Attempt to open file and create a writer.
        let file = File::options()
            .read(false)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&vm_stderr)?;
        Box::new(file)
    } else {
        // Standard error was not set to a file. Fallback to stderr.
        Box::new(std::io::stderr())
    };
    Ok(file_writer)
}

/// Builds an input callback that delivers messages from the Linux daemon to the virtual machine.
///
/// # Parameters
///
/// * `input_queue` - Channel used to receive emulated port-I/O requests originating from the
///   Linux daemon.
///
/// # Returns
///
/// A boxed closure compatible with the VMM's stdin handler implementation.
pub fn build_input_fn(mut input_queue: Receiver<Message>) -> Box<StdinFn> {
    // Input function used for emulating I/O port reads.
    #[cfg(not(feature = "hyperlight"))]
    let input = move |guest: &Arc<Mutex<Guest>>,
                      vmem: &Arc<Mutex<VirtualMemory>>,
                      data,
                      size|
          -> Result<()> {
        use std::mem;

        // Check for invalid operand size.
        if size != mem::size_of::<u32>() {
            let reason: String = format!("invalid operand size (size={size:?})");
            error!("input(): {reason}");
            anyhow::bail!(reason);
        }

        match input_queue.blocking_recv() {
            Some(mut msg) => {
                msg.message_type = MessageType::Ikc;
                let mut locked_guest: MutexGuard<'_, Guest> = guest.blocking_lock();
                let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();
                locked_vm.write_bytes(data as u64, &msg.to_bytes())?;
                locked_guest.consume_credit(&mut locked_vm)?;
            },
            // Channel has disconnected.
            None => {
                let reason: String = "channel has been disconnected".to_string();
                error!("input(): {reason}");
                anyhow::bail!(reason);
            },
        }

        Ok(())
    };

    #[cfg(feature = "hyperlight")]
    let input = move || -> Result<Vec<u8>, hyperlight_host::HyperlightError> {
        match input_queue.blocking_recv() {
            Some(mut msg) => {
                Guest::consume_credit()?;
                msg.message_type = MessageType::Ikc;
                Ok(msg.to_bytes().to_vec())
            },

            // Channel has disconnected.
            None => {
                let reason: String = "channel has been disconnected".to_string();
                error!("input(): {reason}");
                Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
            },
        }
    };

    Box::new(input)
}

///
/// # Description
///
/// Builds an output callback that forwards messages emitted by the virtual machine.
///
/// # Parameters
///
/// - `queue` - Channel used to forward messages emitted by the virtual machine to the Linux daemon.
///
/// # Returns
///
/// A boxed closure compatible with the VMM's stdout handler implementation.
///
pub fn output_fn(queue: Sender<Message>) -> Box<StdoutFn> {
    // Output function used for emulating I/O port writes.
    #[cfg(not(feature = "hyperlight"))]
    let output = move |vm: &Arc<Mutex<VirtualMemory>>, data| -> Result<()> {
        use std::mem;

        // Write to the standard output device.
        let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
        trace!("output(): reading message from user VM");
        vm.blocking_lock().read_bytes(data as u64, &mut bytes)?;

        let message: Message = match Message::try_from_bytes(bytes) {
            Ok(message) => message,
            Err(err) => {
                let reason: String = format!("failed to parse message: {err:?}");
                error!("output(): {reason}");
                anyhow::bail!(reason);
            },
        };

        trace!("output(): forwarding message to system VM");
        if let Err(e) = queue.blocking_send(message) {
            let reason: String = format!("failed to send message: {e:?}");
            error!("output(): {reason}");
            anyhow::bail!(reason);
        }

        trace!("output(): message forwarded to system VM");

        Ok(())
    };

    #[cfg(feature = "hyperlight")]
    let output = move |data: Vec<u8>| -> Result<i32, hyperlight_host::HyperlightError> {
        let bytes: &[u8] = data.as_slice();
        let expected_length: usize = ::core::mem::size_of::<Message>();
        let payload: [u8; ::core::mem::size_of::<Message>()] = match bytes.try_into() {
            Ok(value) => value,
            Err(_) => {
                let reason: String = format!(
                    "failed to convert payload: expected {} bytes, got {}",
                    expected_length,
                    bytes.len()
                );
                error!("output(): {}", reason);
                return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(
                    reason,
                )));
            },
        };

        let message: Message = match Message::try_from_bytes(payload) {
            Ok(message) => message,
            Err(err) => {
                let reason: String = format!("failed to parse message: {:?}", err);
                error!("output(): {}", reason);
                return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(
                    reason,
                )));
            },
        };

        if let Err(e) = queue.blocking_send(message) {
            let reason: String = format!("failed to send message: {:?}", e);
            error!("output(): {}", reason);
            return Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
        }

        match i32::try_from(data.len()) {
            Ok(length) => Ok(length),
            Err(_) => {
                let reason: String =
                    format!("failed to convert payload length {} to i32", data.len());
                error!("output(): {}", reason);
                Err(hyperlight_host::HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
            },
        }
    };

    Box::new(output)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::get_stderr_writer;
    use ::anyhow::{
        Result as AnyResult,
        anyhow,
    };
    use ::std::{
        env,
        fs,
        fs::Metadata,
        io::Write as IoWrite,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    fn unique_log_path(suffix: &str) -> AnyResult<(String, PathBuf)> {
        let mut path: PathBuf = env::temp_dir();
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| anyhow!("failed to compute timestamp (error={:?})", error))?
            .as_nanos();
        let file_name: String = format!("nanvix-uservm-{suffix}-{nanos}.log");
        path.push(&file_name);
        Ok((path.to_string_lossy().into_owned(), path))
    }

    #[test]
    fn get_stderr_writer_creates_target_file() -> AnyResult<()> {
        let (path_str, path_buf): (String, PathBuf) = unique_log_path("stderr")?;

        {
            let mut writer: Box<dyn Write + Send> = get_stderr_writer(Some(path_str.clone()))?;
            writer.write_all(b"hello stderr")?;
            writer.flush()?;
        }

        let metadata: Metadata = fs::metadata(&path_buf)?;
        assert!(metadata.is_file(), "expected log path to be a regular file");
        assert!(metadata.len() >= b"hello stderr".len() as u64, "log file is unexpectedly empty");

        fs::remove_file(path_buf).ok();
        Ok(())
    }

    #[test]
    fn get_stderr_writer_errors_when_directory_missing() {
        let nanos: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("timestamp")
            .as_nanos();
        let mut file_path: PathBuf = env::temp_dir();
        file_path.push(format!("nanvix-uservm-missing-{nanos}"));
        file_path.push("stderr.log");

        let result: Result<Box<dyn Write + Send>> =
            get_stderr_writer(Some(file_path.to_string_lossy().into_owned()));
        assert!(result.is_err(), "expected failure when parent directory does not exist");
    }
}
