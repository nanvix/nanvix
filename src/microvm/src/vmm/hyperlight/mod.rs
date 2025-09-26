// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

use crate::{
    io_thread::IoThread,
    memory_thread,
    orchestrator::{
        IoControlCommand,
        IoControlResponse,
        MemoryControlCommand,
        MemoryControlResponse,
        Orchestrator,
        VcpuControlCommand,
        VcpuControlResponse,
    },
};
use ::anyhow::Result;
use ::hyperlight_host::{
    GuestBinary,
    HyperlightError,
    UninitializedSandbox,
    mem::{
        memory_region::MemoryRegionFlags,
        mgr::SandboxMemoryManager,
        shared_mem::ExclusiveSharedMemory,
    },
    sandbox::{
        SandboxConfiguration,
        uninitialized::{
            GuestBlob,
            GuestEnvironment,
        },
    },
};
use ::std::{
    fs::File,
    io::Write,
    sync::{
        Arc,
        Mutex,
        OnceLock,
        mpsc::{
            self,
            TryRecvError,
        },
    },
    thread::JoinHandle,
};
use ::sys::ipc::{
    Message,
    MessageType,
};
use ::syscomm::SocketStream;
use ::syslog::{
    debug,
    error,
    info,
};

// ==================================================================================================
// Globals
// ==================================================================================================

static VMEM: OnceLock<Arc<Mutex<SandboxMemoryManager<ExclusiveSharedMemory>>>> = OnceLock::new();

//==================================================================================================
// Structure
//==================================================================================================

pub struct Vmm {
    io_thread: Option<JoinHandle<Result<()>>>,
    memory_thread: JoinHandle<Result<()>>,
    // FIXME (#1010): without a mechanism to interrupt a HL sandbox we cannot join() the vCPU
    // thread handle, so we leave it temporarily unused.
    _vcpu_thread: JoinHandle<Result<u16>>,
    orchestrator: Orchestrator,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    ///
    /// # Description
    ///
    /// This function instantiates and runs the virtual machine monitor (VMM) with the given arguments.
    ///
    /// # Parameters
    ///
    /// - `memory_size`: The memory size for the virtual machine in bytes.
    /// - `kernel_filename`: The path to the kernel file to be loaded into the virtual machine.
    /// - `initrd_filename`: An optional path to the initial RAM disk (initrd) file.
    /// - `initrd_args`: Optional arguments to be passed to the initrd.
    /// - `stderr`: An optional path to a file where the virtual machine's standard error output will be written.
    /// - `system_vm_stream`: An optional connection to the system VM for communication with the virtual machine.
    /// - `control_plane_stream`: An optional connection to the nanvixd control-plane.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn spawn(
        memory_size: usize,
        kernel_filename: &str,
        initrd_filename: Option<String>,
        _initrd_args: Option<String>,
        stderr: Option<String>,
        system_vm_stream: Option<SocketStream>,
        control_plane_stream: Option<SocketStream>,
    ) -> Result<u16> {
        crate::timer!("vmm_creation");

        // TODO (#1009): all IO-related channels should be Optional and only be initialized if
        // io_enabled is true.
        let io_enabled: bool = system_vm_stream.is_some() || control_plane_stream.is_some();

        // io/memory/vcpu_control_rx/tx are channels owned by the VMM and transfer control messages.
        // *_thread_control_rx/tx are channels owned by the threads that transfer control messages.
        let (vcpu_thread_stdout_tx, io_thread_data_rx) = mpsc::channel::<Message>();
        let (io_thread_data_tx, memory_thread_data_rx) = mpsc::channel::<Message>();
        let (memory_thread_data_tx, vcpu_thread_stdin_rx) = mpsc::channel::<Message>();
        let (io_thread_control_tx, io_control_rx) = mpsc::channel::<IoControlCommand>();
        let (io_control_tx, io_thread_control_rx) = mpsc::channel::<IoControlResponse>();
        let (memory_control_tx, memory_thread_control_rx) = mpsc::channel::<MemoryControlCommand>();
        let (memory_thread_control_tx, memory_control_rx) =
            mpsc::channel::<MemoryControlResponse>();
        let (vcpu_control_tx, _vcpu_thread_control_rx) = mpsc::channel::<VcpuControlCommand>();
        let (vcpu_thread_control_tx, vcpu_control_rx) = mpsc::channel::<VcpuControlResponse>();

        // Spawn I/O thread if we have any external stream to monitor.
        let io_thread: Option<JoinHandle<Result<()>>> = if io_enabled {
            Some(IoThread::spawn(
                system_vm_stream,
                io_thread_data_rx,
                io_thread_data_tx,
                io_thread_control_tx,
                io_thread_control_rx,
                control_plane_stream,
            ))
        } else {
            None
        };

        // Required values for heap and stack sizes to be used by the kernel.
        let heap_size = 4 * 1024 * 1024;
        let stack_size = 4 * 1024;

        let mut config: SandboxConfiguration = SandboxConfiguration::default();
        config.set_heap_size(heap_size);
        config.set_stack_size(stack_size);

        let file_writer = Self::get_stderr_writer(stderr.clone())?;

        let writer_fn = move |s: String| -> Result<i32, HyperlightError> {
            let mut file_writer = file_writer.lock().unwrap();
            file_writer.write_all(s.as_bytes())?;
            Ok(s.len() as i32)
        };

        let guest_env = if let Some(initrd_filename) = initrd_filename {
            match std::fs::read(&initrd_filename) {
                Ok(bytes) => {
                    let actual_size = bytes.len();
                    debug!("initrd: {} bytes", actual_size);

                    let kernel_size = std::fs::metadata(kernel_filename)
                        .map(|m| m.len() as usize)
                        .map_err(|e| {
                            anyhow::anyhow!("failed to read kernel file metadata: {}", e)
                        })?;

                    let initrd_bytes = std::fs::read(&initrd_filename)?;
                    let initrd_size = initrd_bytes.len();

                    // PEB, I/O buffers, host fxn defs, guard pages, etc.
                    let reserved_pages = 11 * 4096;

                    let used_memory = kernel_size
                        + initrd_size
                        + (heap_size + stack_size) as usize
                        + reserved_pages;

                    if memory_size <= used_memory {
                        return Err(anyhow::anyhow!(
                            "Not enough memory ({} bytes used, {} bytes total)",
                            used_memory,
                            memory_size
                        ));
                    }

                    let padding_size = memory_size - used_memory;

                    // Create a new vector with size header + original data + padding
                    let mut padded_bytes =
                        Vec::with_capacity(::config::hyperlight::INITRD_SIZE_BYTES + actual_size);

                    // Write the actual size as first INITRD_SIZE_BYTES-bytes (little-endian)
                    padded_bytes.extend_from_slice(&(actual_size as u64).to_le_bytes());

                    // Add the actual initrd data
                    padded_bytes.extend_from_slice(&bytes);

                    debug!(
                        "initrd with padding: {} bytes total (8 byte header + {} bytes data + {} \
                         bytes padding)",
                        padded_bytes.len(),
                        actual_size,
                        padding_size
                    );

                    // Box the data to extend its lifetime
                    let boxed_data = padded_bytes.into_boxed_slice();
                    let data_ref: &'static [u8] = Box::leak(boxed_data);

                    GuestEnvironment {
                        guest_binary: GuestBinary::FilePath(kernel_filename.to_string()),
                        init_data: Some(GuestBlob {
                            data: data_ref,
                            permissions: MemoryRegionFlags::READ
                                | MemoryRegionFlags::WRITE
                                | MemoryRegionFlags::EXECUTE,
                        }),
                        extra_memory: Some(padding_size.try_into().unwrap()),
                    }
                },
                Err(err) => {
                    let reason: String = format!("failed to read initrd file: {:?}", err);
                    error!("initrd(): {}", reason);
                    return Err(anyhow::anyhow!("initrd(): {}", reason));
                },
            }
        } else {
            GuestEnvironment::new(GuestBinary::FilePath(kernel_filename.to_string()), None)
        };

        // Creates Hyperlight sandbox.
        let mut sandbox = UninitializedSandbox::new(guest_env, Some(config))?;
        let manager = Arc::new(Mutex::new(sandbox.mgr.unwrap_mgr().clone()));
        VMEM.set(manager).map_err(|_| {
            anyhow::anyhow!("Failed to set VMEM: already initialized or not available")
        })?;
        sandbox.register_print(writer_fn)?;

        sandbox.register("VmbusWrite", move |data: Vec<u8>| -> Result<i32, HyperlightError> {
            let bytes = data.as_slice();
            let message: Message = match Message::try_from_bytes(
                bytes.try_into().expect("slice with incorrect length"),
            ) {
                Ok(message) => message,
                Err(err) => {
                    let reason: String = format!("failed to parse message: {:?}", err);
                    error!("output(): {}", reason);
                    return Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
                },
            };

            if let Err(e) = vcpu_thread_stdout_tx.send(message) {
                let reason: String = format!("failed to send message: {:?}", e);
                error!("output(): {}", reason);
                return Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)));
            }

            Ok(data.len() as i32)
        })?;

        sandbox.register("VmbusRead", move || -> Result<Vec<u8>, HyperlightError> {
            match vcpu_thread_stdin_rx.try_recv() {
                Ok(mut msg) => {
                    consume_credit()?;
                    msg.message_type = MessageType::Ikc;
                    Ok(msg.to_bytes().to_vec())
                },
                // No message available.
                Err(TryRecvError::Empty) => {
                    let empty_message = Message::default();
                    Ok(empty_message.to_bytes().to_vec())
                },
                // Channel has disconnected.
                Err(TryRecvError::Disconnected) => {
                    let reason: String = "channel has been disconnected".to_string();
                    error!("input(): {}", reason);
                    Err(HyperlightError::AnyhowError(anyhow::Error::msg(reason)))
                },
            }
        })?;

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread: JoinHandle<Result<(), anyhow::Error>> = memory_thread::spawn(
            memory_thread_data_rx,
            memory_thread_data_tx,
            memory_thread_control_rx,
            memory_thread_control_tx,
            add_credit,
        );

        let vcpu_thread: JoinHandle<Result<u16>> = std::thread::spawn(move || {
            match sandbox.evolve() {
                Ok(res) => anyhow::bail!("Expected DEFAULT_VMM_SHUTDOWN_CMD, got: {:#?}", res),
                Err(err) => {
                    // note: this is a bit of a hack to check for the shutdown command.
                    if !err
                        .to_string()
                        .contains(&::config::hyperlight::DEFAULT_VMM_SHUTDOWN_CMD.to_string())
                    {
                        anyhow::bail!("Failed to run VMM: {}", err);
                    }
                },
            }

            // Send shutdown message to VMM thread.
            vcpu_thread_control_tx.send(VcpuControlResponse::Shutdown)?;

            // TODO: return the exit status code when supported.
            Ok(0)
        });

        let orchestrator = Orchestrator::new(
            io_enabled,
            io_control_rx,
            io_control_tx,
            memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            Box::new(pause_microvm),
            Box::new(resume_microvm),
            Box::new(|| Ok(())), // TODO: create_snapshot https://github.com/nanvix/nanvix/issues/947
        );

        let mut vmm: Vmm = Self {
            io_thread,
            memory_thread,
            _vcpu_thread: vcpu_thread,
            orchestrator,
        };

        // Main VMM loop.
        vmm.orchestrator.run()?;

        // Join all auxiliary threads once the orchestrator has finished running. Do not bail if we
        // fail to join, as we are already shutting down.
        if let Err(e) = vmm.memory_thread.join() {
            error!("spawn(): error joining memory thread (error={e:?})");
        }
        if let Some(io_thread) = vmm.io_thread {
            // FIXME (#1004): support graceful shutdown of the IO thread.
            let _ = io_thread.join();
        }

        /* FIXME (#1010): without a mechanism to interrupt a running hyperlight sandbox, we cannot
         * reliably join the vCPU thread. Instead, we rely on the JoinHandle being dropped once
         * the orchestrator has finished running.
        match vmm.vcpu_thread.join() {
            Ok(exit_code) => exit_code,
            Err(e) => {
                let reason: String = format!("failed to join vCPU thread (error={e:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        }
        */

        // FIXME (#1010): the vCPU thread already returns 0 always, but we will be able to remove
        // this line once we can join the vCPU thread.
        Ok(0)
    }

    ///
    /// # Description
    ///
    /// Obtains a buffered writer for the virtual machine's standard error device. If the standard
    /// error device is set to a file, the function attempts to open the file and create a buffered
    /// writer. If the standard error device is not set to a file, the function falls back to stderr.
    ///
    /// # Parameters
    ///
    /// * `vm_stderr` - The path to the file where the standard error device is set.
    ///
    /// # Returns
    ///
    /// On success, the function returns a buffered writer for the virtual machine's standard error
    ///
    fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Arc<Mutex<File>>> {
        // Obtain a buffered writer for the virtual machine's standard error device.
        let file_writer: Arc<Mutex<File>> = if let Some(vm_stderr) = vm_stderr {
            // Standard error was set to a file. Attempt to open file and create a writer.
            let file = File::options()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&vm_stderr)?;
            debug!("stderr: {:?}", file);
            Arc::new(Mutex::new(file))
        } else {
            // Standard error was not set to a file. Fallback to stderr.
            Arc::new(Mutex::new(File::create("/dev/stderr")?))
        };
        Ok(file_writer)
    }
}

// Adds a credit to the virtual machine's credit pool.
fn add_credit() -> Result<()> {
    VMEM.get()
        .and_then(|vmem| vmem.lock().ok())
        .map(|mut vmem| -> Result<()> {
            let credits_offset = vmem.get_guest_credits_offset();
            let mut credit = vmem.get_shared_mem_mut().read::<u64>(credits_offset)?;
            credit += 1;
            vmem.get_shared_mem_mut()
                .write::<u64>(credits_offset, credit)?;

            info!("Adding credit: {}", credit);
            Ok(())
        })
        .ok_or(anyhow::anyhow!("VMEM is not initialized"))?
}

// Consumes a credit from the virtual machine's credit pool.
fn consume_credit() -> Result<()> {
    VMEM.get()
        .and_then(|vmem| vmem.lock().ok())
        .map(|mut vmem| -> Result<()> {
            let credits_offset = vmem.get_guest_credits_offset();
            let mut credit = vmem.get_shared_mem_mut().read::<u64>(credits_offset)?;

            if credit == 0 {
                return Err(anyhow::anyhow!("No credit available to consume"));
            }

            credit -= 1;
            vmem.get_shared_mem_mut()
                .write::<u64>(credits_offset, credit)?;

            info!("Consuming credit: {}", credit);
            Ok(())
        })
        .ok_or(anyhow::anyhow!("VMEM is not initialized"))?
}

///
/// # Description
///
/// Requests the kernel to pause the virtual machine's execution by writing to a specific register.
///
/// # Returns
///
/// On success, returns empty. Otherwise, returns an error.
///
fn pause_microvm() -> Result<()> {
    Ok(()) // TODO: https://github.com/nanvix/nanvix/issues/791
}

///
/// # Description
///
/// Writes to a specific kernel register that execution should not be paused.
///
/// # Returns
///
/// On success, returns empty. Otherwise, returns an error.
///
fn resume_microvm() -> Result<()> {
    Ok(()) // TODO: https://github.com/nanvix/nanvix/issues/791
}
