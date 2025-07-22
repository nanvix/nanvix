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
    time::Instant,
};
use ::sys::ipc::{
    Message,
    MessageType,
};

//==================================================================================================
// Constants
//==================================================================================================

// This value was chosen so it catches issues without polluting the logs with too many warnings.
const TIMEOUT_WARNING_INTERVAL_IN_MS: usize = 10;

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
    control_output_tx: Sender<ControlCommandResponse>,
    orchestrator_state: OrchestratorState,
}

///
/// # Description
///
/// States relating to snapshots functionality. Snapshots may be loaded at PreBoot, and created at Paused.
///
#[derive(PartialEq)]
enum OrchestratorState {
    PreBoot,
    Running,
    Paused,
}

///
/// # Description
///
/// Control plane commands.
///
#[derive(PartialEq)]
pub enum ControlCommand {
    _StartMicroVm,
    _LoadSnapshotAndRun,
    _PauseMicroVm,
    _PauseAndCreateSnapshot,
    _CreateSnapshot,
    _ResumeMicroVm,
    LinuxDaemonFlushed,
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
            control_output_tx,
            orchestrator_state: OrchestratorState::PreBoot,
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

    ///
    /// # Description
    ///
    /// Attempts to handle a command from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn handle_command(&mut self) -> Result<()> {
        match self.control_input_rx.try_recv() {
            Ok(command) => match command {
                ControlCommand::_StartMicroVm => {
                    if self.orchestrator_state == OrchestratorState::PreBoot {
                        // TODO: separate starting logic from `spawn()` and put it here
                        // This TODO could be done right now, but it's a major refactor.
                        self.orchestrator_state = OrchestratorState::Running;
                        trace!("OrchestratorState: PreBoot -> Running");
                    }
                    Ok(())
                },
                ControlCommand::_LoadSnapshotAndRun => {
                    if self.orchestrator_state == OrchestratorState::PreBoot {
                        // TODO: load snapshot
                        // This TODO requires being able to create snapshots.

                        // The Linux daemon should send messages to PreBoot VMMs by default,
                        // so there's no need to tell it to resume sending messages.

                        if let Err(e) = self.resume_microvm() {
                            let reason: String =
                                format!("LoadSnapshotAndRun: failed to resume microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("OrchestratorState: PreBoot -> Running");
                    }
                    Ok(())
                },
                ControlCommand::_PauseMicroVm => {
                    if self.orchestrator_state == OrchestratorState::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseMicroVm: failed to pause microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                ControlCommand::_PauseAndCreateSnapshot => {
                    if self.orchestrator_state == OrchestratorState::Running {
                        if let Err(e) = self.pause_protocol() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to pause microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        if let Err(e) = self.create_snapshot() {
                            let reason: String =
                                format!("PauseAndCreateSnapshot: failed to create snapshot: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                ControlCommand::_CreateSnapshot => {
                    if self.orchestrator_state == OrchestratorState::Paused {
                        if let Err(e) = self.create_snapshot() {
                            let reason: String =
                                format!("CreateSnapshot: failed to create snapshot: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                    }
                    Ok(())
                },
                ControlCommand::_ResumeMicroVm => {
                    if self.orchestrator_state == OrchestratorState::Paused {
                        // TODO: tell linuxd it's fine to send more messages
                        // This TODO requires having a control plane connection with linuxd
                        if let Err(e) = self.resume_microvm() {
                            let reason: String =
                                format!("ResumeMicroVm: failed to resume microvm: {e:?}");
                            error!("handle_command(): {reason}");
                            anyhow::bail!(reason);
                        }
                        trace!("OrchestratorState: Paused -> Running");
                    }
                    Ok(())
                },
                ControlCommand::LinuxDaemonFlushed => {
                    // NOTE: this will be unreachable once the communication is fully implemented
                    // `LinuxDaemonFlushed` should only be sent in the middle of `pause_protocol`.
                    // In fact, it should already be unreachable, but it cannot be tested ATM.
                    Ok(())
                },
            },
            Err(TryRecvError::Empty) => Ok(()),
            Err(TryRecvError::Disconnected) => {
                let reason: String =
                    ("disconnected from the input control command channel").to_string();
                error!("handle_command(): {reason}");
                anyhow::bail!(reason);
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to pause the execution of the MicroVM and the communication with the Linux daemon.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned.
    ///
    fn pause_protocol(&mut self) -> Result<()> {
        // TODO: pause MicroVM (Running -> Paused)
        // and tell linuxd to flush (Running -> Flushing)
        // This TODO requires pausing the vCPU and a control plane communication with linuxd
        trace!("MicroVM paused");
        // Flush output to linuxd
        self.control_output_tx
            .send(ControlCommandResponse::FlushOutput)?;
        // TODO: tell linuxd to stop sending messages (Flushing -> Paused)
        // TODO: get a response from linuxd
        // These TODOs require a control plane communication with linuxd
        self.control_output_tx
            .send(ControlCommandResponse::FlushInput)?;
        self.receive_linux_daemon_flushed()?;
        self.orchestrator_state = OrchestratorState::Paused;
        self.control_output_tx
            .send(ControlCommandResponse::MicroVmPaused)?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a `LinuxDaemonFlushed` message from the control input.
    ///
    /// # Returns
    ///
    /// Upon success, empty is returned. Otherwise, an error is returned instead.
    ///
    fn receive_linux_daemon_flushed(&mut self) -> Result<()> {
        // Check how long it takes to receive a response
        let start: Instant = Instant::now();
        let mut counter: usize = 1;
        // Loop until `LinuxDaemonFlushed` arrives.
        // Different kinds of messages can be ignored,
        // as they wouldn't do anything while the VMM is pausing.
        while match self.control_input_rx.try_recv() {
            Ok(command) => command != ControlCommand::LinuxDaemonFlushed,
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                let reason: String = "the vmm has disconnected".to_string();
                error!("receive_linux_daemon_flushed(): {reason}");
                anyhow::bail!(reason)
            },
        } {
            // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
            let elapsed_time: usize = start.elapsed().as_millis() as usize;
            if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                warn!(
                    "{}ms have passed waiting for `LinuxDaemonFlushed`",
                    TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                );
                counter += 1;
            }
        }
        Ok(())
    }

    fn create_snapshot(&self) -> Result<()> {
        // TODO: create snapshot
        trace!("Snapshot created");
        self.control_output_tx
            .send(ControlCommandResponse::SnapshotCreated)?;
        Ok(())
    }

    fn resume_microvm(&self) -> Result<()> {
        // TODO: resume MicroVM
        trace!("MicroVM resumed");
        Ok(())
    }
}
