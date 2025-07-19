// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::module_inception)]

//==================================================================================================
// Modules
//==================================================================================================

mod io;
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
    vmm::microvm::{
        io::{
            ControlCommand,
            ControlCommandResponse,
            IoThread,
        },
        kvm::vmem::VirtualMemory,
        microvm::MicroVm,
    },
};
use ::anyhow::Result;
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
            TryRecvError,
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
    _gateway_tx: Sender<Message>,
    io_thread: Option<JoinHandle<Result<()>>>,
    _memory_thread: JoinHandle<Result<()>>,
    vcpu_thread: JoinHandle<Result<u16>>,
    _microvm: Arc<Mutex<MicroVm>>,
    control_input_rx: Receiver<ControlCommand>,
    _control_output_tx: Sender<ControlCommandResponse>,
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
    /// * `args` - Arguments for the virtual machine monitor.
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

        let (vm_tx, gateway_rx) = mpsc::channel::<Message>();
        let (gateway_tx, memory_thread_rx) = mpsc::channel::<Message>();
        let (memory_thread_tx, vm_rx) = mpsc::channel::<Message>();
        let (control_input_tx, control_input_rx) = mpsc::channel::<ControlCommand>();
        let (control_output_tx, control_output_rx) = mpsc::channel::<ControlCommandResponse>();

        // Spawn I/O thread.
        let io_thread: Option<JoinHandle<Result<()>>> = gateway_conn.map(|conn| {
            IoThread::spawn(
                conn,
                gateway_rx,
                gateway_tx.clone(),
                control_input_tx,
                control_output_rx,
            )
        });

        // Input function used for emulating I/O port reads.
        let input: Box<microvm::InputFn> = Self::build_input_fn(vm_rx);

        // Output function used for emulating I/O port writes.
        let output: Box<microvm::OutputFn> =
            Self::build_output_fn(Self::get_stderr_writer(stderr.clone())?, vm_tx);

        let mut microvm: MicroVm = MicroVm::new(memory_size, input, output)?;

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

        let microvm: Arc<Mutex<MicroVm>> = Arc::new(Mutex::new(microvm));

        let vmem: Arc<Mutex<VirtualMemory>> = microvm
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .vmem();

        vmem.lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .reset_credits()?;

        // Create a thread that reads from vm_rx and writes to vm_rx2.
        let memory_thread_tx: Sender<Message> = memory_thread_tx.clone();
        let memory_thread: JoinHandle<Result<(), anyhow::Error>> = std::thread::spawn(move || {
            loop {
                match memory_thread_rx.try_recv() {
                    Ok(mut msg) => {
                        profiler::timestamp_message!(
                            &mut msg.payload,
                            mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                                + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
                        );
                        if let Err(e) = memory_thread_tx.send(msg) {
                            let reason: String = format!("failed to send message: {e:?}");
                            error!("memory_thread(): {reason}");
                            continue;
                        }
                        vmem.lock()
                            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                            .add_credit()?;
                    },
                    Err(TryRecvError::Disconnected) => {
                        // When the guest finishes , the vCPU thread will disconnect from this
                        // thread. This situation is normal and should not create an error log.
                        debug!("memory_thread(): channel has been disconnected");
                        break Ok(());
                    },
                    Err(TryRecvError::Empty) => {
                        // No message available.
                    },
                }
            }
        });

        let microvm_clone: Arc<Mutex<MicroVm>> = microvm.clone();
        let vcpu_thread: JoinHandle<Result<u16>> = std::thread::spawn(move || {
            microvm_clone
                .lock()
                .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                .run()
        });

        let mut vmm: Vmm = Self {
            _gateway_tx: gateway_tx,
            io_thread,
            _memory_thread: memory_thread,
            vcpu_thread,
            _microvm: microvm,
            control_input_rx,
            _control_output_tx: control_output_tx,
        };

        if vmm.io_thread.is_some() {
            while !vmm.vcpu_thread.is_finished() {
                vmm.handle_command()?;
            }
        }

        match vmm.vcpu_thread.join() {
            Ok(exit_code) => exit_code,
            Err(e) => {
                let reason: String = format!("failed to join vCPU thread (error={e:?})");
                error!("run(): {reason}");
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

    fn handle_command(&mut self) -> Result<()> {
        match self.control_input_rx.try_recv() {
            Ok(_command) => Ok(()), // TODO: handle commands.
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String =
                    ("disconnected from the input control command channel").to_string();
                error!("handle_command(): {reason}");
                anyhow::bail!(reason);
            },
        }
    }
}
