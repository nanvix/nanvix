// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::module_inception)]

//==================================================================================================
// Modules
//==================================================================================================

mod kvm;
mod microvm;
mod pal;

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
extern crate kvm_bindings;
#[cfg(target_os = "linux")]
extern crate kvm_ioctls;

use crate::{
    Gateway,
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
    vmm::microvm::{
        kvm::vmem::VirtualMemory,
        microvm::MicroVm,
    },
};
use ::anyhow::Result;
use ::libc::pthread_self;
use ::std::{
    fs::File,
    io::Write,
    mem,
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        mpsc,
        mpsc::{
            Receiver,
            RecvError,
            Sender,
        },
    },
    thread::JoinHandle,
};
use ::sys::ipc::{
    Message,
    MessageType,
};

//==================================================================================================
// Structure
//==================================================================================================

pub struct Vmm {
    io_thread: Option<JoinHandle<Result<()>>>,
    _memory_thread: JoinHandle<Result<()>>,
    vcpu_thread: JoinHandle<Result<u16>>,
    _microvm: MicroVm,
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
    /// - `gateway_conn`: An optional connection to the gateway for communication with the virtual machine.
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
        initrd_args: Option<String>,
        stderr: Option<String>,
        gateway_conn: Option<Gateway>,
    ) -> Result<u16> {
        crate::timer!("vmm_creation");

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
        let (vcpu_control_tx, vcpu_thread_control_rx) = mpsc::channel::<VcpuControlCommand>();
        let (vcpu_thread_control_tx, vcpu_control_rx) = mpsc::channel::<VcpuControlResponse>();

        // Spawn I/O thread.
        let io_thread: Option<JoinHandle<Result<()>>> = gateway_conn.map(|conn| {
            IoThread::spawn(
                conn,
                io_thread_data_rx,
                io_thread_data_tx,
                io_thread_control_tx,
                io_thread_control_rx,
            )
        });

        // Input function used for emulating I/O port reads.
        let input: Box<microvm::InputFn> = Self::build_input_fn(vcpu_thread_stdin_rx);

        // Output function used for emulating I/O port writes.
        let output: Box<microvm::OutputFn> =
            Self::build_output_fn(Self::get_stderr_writer(stderr.clone())?, vcpu_thread_stdout_tx);

        let mut microvm: MicroVm = MicroVm::new(
            memory_size,
            input,
            output,
            vcpu_thread_control_rx,
            vcpu_thread_control_tx,
        )?;

        let rip: u64 = microvm.load_kernel(kernel_filename)?;
        if let Some(ref initrd_filename) = initrd_filename {
            microvm.load_initrd(initrd_filename)?;

            // Write arguments to the virtual machine. For now, just pass the initrd filename.
            let mut args: String = initrd_filename
                .split('/')
                .next_back()
                .unwrap_or(initrd_filename)
                .to_string();

            // Add initrd arguments if provided.
            if let Some(ref initrd_args) = initrd_args {
                args.push_str(&format!(" {initrd_args}"));
            }

            microvm.write_args(&args)?;
        }

        microvm.reset(rip)?;

        let vmem: Arc<Mutex<VirtualMemory>> = microvm.vmem();

        vmem.lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .reset_credits()?;

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let vmem_pause_microvm: Arc<Mutex<VirtualMemory>> = vmem.clone();
        let vmem_resume_microvm: Arc<Mutex<VirtualMemory>> = vmem.clone();
        let memory_thread: JoinHandle<Result<(), anyhow::Error>> = memory_thread::spawn(
            memory_thread_data_rx,
            memory_thread_data_tx,
            memory_thread_control_rx,
            memory_thread_control_tx,
            move || {
                vmem.lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                    .add_credit()
            },
            move || {
                vmem_pause_microvm
                    .lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
                    .write_bytes(
                        ::config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as u64,
                        &::config::microvm::PAUSE_REQUEST.to_le_bytes(),
                    )
            },
            move || {
                vmem_resume_microvm
                    .lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
                    .write_bytes(
                        ::config::microvm::DEFAULT_MICROVM_CTRL_PAUSE_REQUESTED as u64,
                        &::config::microvm::RUNNING.to_le_bytes(),
                    )
            },
        );

        let mut microvm_clone: MicroVm = microvm.clone();
        let vcpu_thread: JoinHandle<Result<u16>> = std::thread::spawn(move || {
            // Store the tid so that the caller can send signals to the vCPU thread.
            // SAFETY: we are calling pthread_self() right after creating the thread so this is
            // safe.
            let pthread_id: libc::pthread_t = unsafe { pthread_self() };
            microvm_clone
                .send_tid(pthread_id)
                .map_err(|e| anyhow::anyhow!("failed to send tid {e:?}"))?;
            microvm_clone.run()
        });

        // Wait right after spawning the vCPU thread such that we populate the pthread id holder
        // before actually starting the vCPU.
        match vcpu_control_rx.recv() {
            Ok(VcpuControlResponse::Tid(tid)) => trace!("Received vCPU thread tid: {tid}"),
            Ok(response) => unreachable!(
                "the first message sent on this channel is always a Tid response ( \
                 response={response:?})"
            ),
            Err(e) => {
                let reason: String = format!("the vCPU thread has disconnected (error={e:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        }

        let orchestrator = Orchestrator::new(
            io_control_rx,
            io_control_tx,
            memory_control_rx,
            memory_control_tx,
            vcpu_control_rx,
            vcpu_control_tx,
            || Ok(()), // TODO: create_snapshot https://github.com/nanvix/nanvix/issues/947
        );

        let mut vmm: Vmm = Self {
            io_thread,
            _memory_thread: memory_thread,
            vcpu_thread,
            _microvm: microvm,
            orchestrator,
        };

        if vmm.io_thread.is_some() {
            while !vmm.vcpu_thread.is_finished() {
                vmm.orchestrator.handle_command()?;
            }
        }

        match vmm.vcpu_thread.join() {
            Ok(exit_code) => exit_code,
            Err(e) => {
                let reason: String = format!("failed to join vCPU thread (error={e:?})");
                error!("spawn(): {reason}");
                anyhow::bail!(reason)
            },
        }
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
    fn get_stderr_writer(vm_stderr: Option<String>) -> Result<Box<dyn Write>> {
        // Obtain a buffered writer for the virtual machine's standard error device.
        let file_writer: Box<dyn Write> = if let Some(vm_stderr) = vm_stderr {
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

    fn build_input_fn(input_queue: Receiver<Message>) -> Box<microvm::InputFn> {
        // Input function used for emulating I/O port reads.
        let input = move |vmem: &Arc<Mutex<VirtualMemory>>, data, size| -> Result<()> {
            // Check for invalid operand size.
            if size != mem::size_of::<u32>() {
                let reason: String = format!("invalid operand size (size={size:?})");
                error!("input(): {reason}");
                anyhow::bail!(reason);
            }

            match input_queue.recv() {
                Ok(mut msg) => {
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    msg.message_type = MessageType::Ikc;
                    let mut locked_vm: MutexGuard<'_, VirtualMemory> = vmem
                        .lock()
                        .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;
                    profiler::timestamp_message!(
                        &mut msg.payload,
                        mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                            + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                    );
                    locked_vm.write_bytes(data as u64, &msg.to_bytes())?;
                    locked_vm.consume_credit().unwrap();
                },
                // Channel has disconnected.
                Err(RecvError) => {
                    let reason: String = "channel has been disconnected".to_string();
                    error!("input(): {reason}");
                    anyhow::bail!(reason);
                },
            }

            Ok(())
        };

        Box::new(input)
    }

    fn build_output_fn(
        mut file_writer: Box<dyn Write>,
        queue: Sender<Message>,
    ) -> Box<microvm::OutputFn> {
        // Output function used for emulating I/O port writes.
        let output = move |vm: &Arc<Mutex<VirtualMemory>>, data, size| -> Result<()> {
            // Parse operand size do determine how to handle the operation.
            if size == 1 {
                // Write to the standard error device.

                // Convert data to a character.
                let ch: char = match char::from_u32(data) {
                    // Valid character.
                    Some(ch) => ch,
                    // Invalid character.
                    None => {
                        let reason: String = format!("invalid character (data={data:?})");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };

                let buf: &[u8] = &[ch as u8];

                file_writer.write_all(buf)?;

                Ok(())
            } else {
                // Write to the standard output device.
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                vm.lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                    .read_bytes(data as u64, &mut bytes)?;

                let mut message: Message = match Message::try_from_bytes(bytes) {
                    Ok(message) => message,
                    Err(err) => {
                        let reason: String = format!("failed to parse message: {err:?}");
                        error!("output(): {reason}");
                        anyhow::bail!(reason);
                    },
                };
                profiler::timestamp_message!(
                    &mut message.payload,
                    std::mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                        + std::mem::offset_of!(syscall::unistd::message::WriteRequest, buffer)
                );

                if let Err(e) = queue.send(message) {
                    let reason: String = format!("failed to send message: {e:?}");
                    error!("output(): {reason}");
                    anyhow::bail!(reason);
                }

                Ok(())
            }
        };

        Box::new(output)
    }
}
