// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::module_inception)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod emulator;
pub mod guest;
pub mod kvm;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    MicroVmArgs,
    guest::Guest,
    kvm::{
        irqchip::{
            IrqChip,
            IrqChipState,
        },
        timer::{
            Timer,
            TimerState,
        },
        vcpu::VirtualProcessorState,
    },
};
#[cfg(target_os = "linux")]
use crate::{
    orchestrator::{
        VcpuControlCommand,
        VcpuControlResponse,
    },
    vmm::emulator::Emulator,
    vmm::microvm::kvm::vcpu::{
        VirtualProcessor,
        VirtualProcessorExitContext,
        VirtualProcessorExitReasonRef,
    },
};
use ::anyhow::Result;
use ::kvm_ioctls::{
    Kvm,
    VmFd,
};
use ::libc::{
    SIGUSR1,
    c_int,
    sigaction,
    sigemptyset,
};
use ::std::{
    ffi::OsStr,
    fs::File,
    io::Write,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
};
use ::sys::error::ErrorCode;
use ::syslog::{
    error,
    trace,
    warn,
};
use ::tokio::{
    runtime::Handle,
    sync::{
        Mutex,
        MutexGuard,
        mpsc::{
            Receiver,
            Sender,
        },
    },
    task,
};

pub use kvm::vmem::VirtualMemory;

//==================================================================================================
// Constants
//==================================================================================================

/// Signal used to interrupt the vCPU thread.
pub const INTERRUPT_SIGNAL: c_int = SIGUSR1;

/// Signal used to kill the vCPU thread.
pub const KILL_SIGNAL: c_int = libc::SIGKILL;

//==================================================================================================
// Thread-Local Variables
//==================================================================================================

thread_local! {
    ///
    /// # Description
    ///
    /// Shutdown flag, set to true when the vCPU thread receives a shutdown signal.
    /// This will prevent the vCPU from entering KVM_RUN again and blocking indefinitely.
    ///
    /// This variable must be thread-safe to enable multiple VMM instances to co-exist in the same
    /// process.
    ///
    static SHUTDOWN: AtomicBool = const { AtomicBool::new(false) };
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a VMM.
///
#[derive(Clone)]
pub struct Vmm {
    /// Guest of the virtual machine.
    guest: Arc<Mutex<Guest>>,
    /// Virtual memory of the virtual machine.
    vmem: Arc<Mutex<VirtualMemory>>,
    /// Virtual processor of the virtual machine.
    vcpu: Arc<Mutex<VirtualProcessor>>,
    /// Wraps fields that don't require individual `Arc<Mutex<_>>`s.
    inner: Arc<Mutex<InteriorMicroVmHandle>>,
}

///
/// # Description
///
/// An internal structure to the VMM that wraps its contents in `Arc<Mutex<_>>`. It allows
/// `MicroVm` to be clonable without wrapping each field in `Arc<Mutex<_>>`.
///
struct InteriorMicroVmHandle {
    /// Handle to the KVM (keep it)
    kvm: Kvm,
    /// Handle to the virtual machine.
    vm: VmFd,
    /// Programmable interrupt controller.
    irqchip: IrqChip,
    /// Programmable interrupt timer.
    timer: Timer,
    /// Emulator of the virtual machine.
    emulator: Emulator,
    /// Channel to receive commands from the VMM.
    control_rx: Receiver<VcpuControlCommand>,
    /// Channel to send control responses to the VMM.
    control_tx: Sender<VcpuControlResponse>,
}

//==================================================================================================
// Types
//==================================================================================================

pub type StdinFn =
    dyn FnMut(&Arc<Mutex<Guest>>, &Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()> + Send;

pub type StdoutFn = dyn FnMut(&Arc<Mutex<VirtualMemory>>, u32) -> Result<()> + Send;

pub type StderrFn = dyn Write + Send;

//==================================================================================================
// Implementations
//==================================================================================================

/// Signal handler for the vCPU thread. We install an empty handler to trigger an -EINTR.
extern "C" fn vcpu_thread_signal_handler(_: i32) {
    SHUTDOWN.with(|shutdown| shutdown.store(true, Ordering::SeqCst));
}

impl Vmm {
    ///
    /// # Description
    ///
    /// Creates a VMM.
    ///
    /// # Parameters
    ///
    /// - `args`: Arguments for creating the VMM.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the VMM that was created. Otherwise, it
    /// returns an error.
    ///
    pub fn new(args: MicroVmArgs) -> Result<Self> {
        trace!("new(): args={:?}", args);

        let mut kvm: Kvm = Kvm::new()?;
        let mut vm: VmFd = kvm.create_vm()?;

        let irqchip: IrqChip = IrqChip::new(&mut kvm, &mut vm)?;
        let timer: Timer = Timer::new(&mut kvm, &mut vm)?;
        let mut vcpu: VirtualProcessor = VirtualProcessor::new(&mut kvm, &mut vm, 0)?;
        let mut vmem: VirtualMemory = VirtualMemory::new(&mut kvm, &mut vm, args.memory_size)?;
        let guest: Arc<Mutex<Guest>> = {
            let mut guest = Guest::default();

            guest.load_kernel(&mut vmem, &args.kernel_filename)?;
            args.initrd_filename
                .as_ref()
                .map(|initrd_filename| {
                    guest.load_initrd(&mut vmem, initrd_filename, args.initrd_args)
                })
                .transpose()?;

            guest.reset(&mut vmem, &mut vcpu)?;

            Arc::new(Mutex::new(guest))
        };

        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(vmem));

        let vcpu: Arc<Mutex<VirtualProcessor>> = Arc::new(Mutex::new(vcpu));

        let emulator: Emulator =
            Emulator::new(guest.clone(), vmem.clone(), args.input, args.output, args.stderr)?;

        Ok(Self {
            guest,
            vmem,
            vcpu,
            inner: Arc::new(Mutex::new(InteriorMicroVmHandle {
                kvm,
                vm,
                irqchip,
                timer,
                emulator,
                control_rx: args.control_rx,
                control_tx: args.control_tx,
            })),
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || {
            let pthread_id: libc::pthread_t = unsafe { libc::pthread_self() };
            Handle::current().block_on(self.send_tid(pthread_id))?;
            self.run()
        })
    }

    /// Install a signal handler on the vCPU thread.
    fn install_signal_handler() {
        // SAFETY: we install a signal handler that is a no-op so this is safe.
        let ret: c_int = unsafe {
            let sig_action: sigaction = sigaction {
                sa_sigaction: vcpu_thread_signal_handler as usize,
                // Empty set to not block any other signals that may happen during signal handling.
                sa_mask: {
                    let mut set: libc::sigset_t = std::mem::zeroed();
                    sigemptyset(&mut set);
                    set
                },
                // No SA_RESTART so that we will trigger a -EINTR.
                sa_flags: 0,
                sa_restorer: None,
            };

            sigaction(INTERRUPT_SIGNAL, &sig_action, std::ptr::null_mut())
        };

        if ret != 0 {
            // Notify the error, but don't fail.
            let errno: i32 = unsafe { *libc::__errno_location() };
            error!("error installing signal handler (errno={errno:?})");
        }
    }

    ///
    /// # Description
    ///
    /// Runs the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the exit status of the virtual machine.
    /// Otherwise, it returns an error.
    ///
    pub fn run(&mut self) -> Result<u16> {
        trace!("run()");

        // Reset shutdown flag from any previous runs.
        SHUTDOWN.with(|shutdown| shutdown.store(false, Ordering::SeqCst));

        // Install a signal handler in the virtual processor's thread.
        Self::install_signal_handler();

        loop {
            // Check shutdown flag before entering KVM_RUN, and blocking indefinitely.
            if SHUTDOWN.with(|shutdown| shutdown.load(Ordering::SeqCst)) {
                let exit_status: u16 = 0;
                Handle::current().block_on(self.handle_shutdown(exit_status));
                break Ok(exit_status);
            }

            let exit_context: VirtualProcessorExitContext = {
                let mut locked_vcpu: MutexGuard<'_, VirtualProcessor> = self.vcpu.blocking_lock();
                // Exit if the vCPU is no longer online.
                if !locked_vcpu.is_online() {
                    break Ok(locked_vcpu.exit_status());
                }
                locked_vcpu.run()
            };

            // Parse exit reason.
            match exit_context.reason_ref() {
                // The guest requested to access an I/O port.
                VirtualProcessorExitReasonRef::PmioAccess(access) => {
                    let exit_status = self
                        .inner
                        .blocking_lock()
                        .emulator
                        .handle_pmio_access(access)?;
                    if let Some(exit_status) = exit_status {
                        if exit_status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD {
                            Handle::current().block_on(self.handle_shutdown(exit_status));

                            break Ok(exit_status);
                        } else {
                            Handle::current().block_on(self.handle_pause())?;
                            trace!("VMM resumed");
                        }
                    }
                },

                // The guest was halted or interrupted, this means we need to power-off.
                VirtualProcessorExitReasonRef::Halt
                | VirtualProcessorExitReasonRef::Interrupted => {
                    let exit_status: u16 = 0;
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReasonRef::Unknown => {
                    break Ok(ErrorCode::IllegalByteSequence.into());
                },
            }
        }
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the virtual memory of the target virtual machine.
    ///
    /// # Returns
    ///
    /// A reference to the virtual memory of the target virtual machine.
    ///
    pub fn vmem(&self) -> Arc<Mutex<VirtualMemory>> {
        self.vmem.clone()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the guest of the virtual machine.
    ///
    /// # Returns
    ///
    /// A reference to the guest of the virtual machine.
    ///
    pub fn guest(&self) -> Arc<Mutex<Guest>> {
        self.guest.clone()
    }

    ///
    /// # Description
    ///
    /// Sends the vCPU thread's tid to the main thread.
    ///
    /// # Parameters
    ///
    /// - `tid`: The vCPU thread's tid.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    async fn send_tid(&self, tid: libc::pthread_t) -> Result<()> {
        Ok(self
            .inner
            .lock()
            .await
            .control_tx
            .send(VcpuControlResponse::Tid(tid))
            .await?)
    }

    ///
    /// # Description
    ///
    /// Strips prefixes and suffixes from the executable filepath, and concatenates it with the
    /// appropriate directory and extensions.
    ///
    /// # Parameters
    ///
    /// - `filepath`: Path to the executable file.
    ///
    /// # Returns
    ///
    /// Returns the filepath to the virtual memory snapshot and the kvm snapshot.
    ///
    fn make_snapshot_paths(filepath: &str) -> (PathBuf, PathBuf) {
        let snapshots_dir: &Path = Path::new("snapshots");

        let stem: &OsStr = Path::new(filepath)
            .file_stem()
            .unwrap_or(OsStr::new("default"));

        let vmem_filepath: PathBuf = snapshots_dir.join(stem).with_extension("vmem");
        let kvm_filepath: PathBuf = snapshots_dir.join(stem).with_extension("kvm.json");
        (vmem_filepath, kvm_filepath)
    }

    ///
    /// # Description
    ///
    /// Acknowledges a pause request and waits for the next command, either `Resume` or `CreateSnapshot`.
    ///
    /// # Returns
    ///
    /// Upon success, return empty. Otherwise, returns an error.
    ///
    async fn handle_pause(&mut self) -> Result<()> {
        self.inner
            .lock()
            .await
            .control_tx
            .send(VcpuControlResponse::Paused)
            .await?;

        match self.inner.lock().await.control_rx.recv().await {
            Some(VcpuControlCommand::Resume) => Ok(()),
            Some(VcpuControlCommand::CreateSnapshot(filepath)) => {
                self.handle_create_snapshot(filepath).await?;
                Ok(())
            },
            // NOTE: Should we add an option for shutting down? Like so:
            // Some(VcpuControlCommand::Shutdown) => self.vcpu.poweroff(0),
            None => {
                let reason: String = "the vmm has disconnected".to_string();
                error!("run(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Helper method to poweroff the vCPU with a given exit status, and send a shutdown message to
    /// the orchestrator thread. If the channel is closed, we still proceed with shutdown as the
    /// orchestrator will handle this via timeout.
    ///
    /// # Arguments
    ///
    /// - `exit_status`: the exit code to set for the vCPU.
    ///
    async fn handle_shutdown(&mut self, exit_status: u16) {
        // Power-off vCPU.
        self.vcpu.lock().await.poweroff(exit_status);

        // Send message to orchestrator thread.
        // If the channel is closed, the orchestrator has already disconnected,
        // but we still need to clean up our side properly.
        match self
            .inner
            .lock()
            .await
            .control_tx
            .send(VcpuControlResponse::Shutdown)
            .await
        {
            Ok(()) => {
                trace!("handle_shutdown(): shutdown notification sent to orchestrator");
            },
            Err(error) => {
                warn!("handle_shutdown(): failed to notify orchestrator thread (error={error:?})");
                // Don't bail as we are shutting down anyway. The orchestrator will detect
                // this via timeout and forcefully terminate the vCPU thread if needed.
            },
        }
    }

    async fn handle_create_snapshot(&self, filepath: String) -> Result<()> {
        match self.create_snapshot(filepath).await {
            Ok(()) => {
                self.inner
                    .lock()
                    .await
                    .control_tx
                    .send(VcpuControlResponse::SnapshotCreated)
                    .await?;
                Ok(())
            },
            Err(error) => {
                self.inner
                    .lock()
                    .await
                    .control_tx
                    .send(VcpuControlResponse::SnapshotCreationFailed)
                    .await?;
                error!("run(): failed to create snapshot: {error:?}");
                Err(error)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Saves the virtual memory and the KVM state to files.
    ///
    /// # Parameters
    ///
    /// - `filepath`: Path to the executable file.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub async fn create_snapshot(&self, filepath: String) -> Result<()> {
        let (vmem_filepath, kvm_filepath) = Self::make_snapshot_paths(&filepath);

        if let Err(e) = self.vmem.lock().await.save_snapshot(&vmem_filepath) {
            let reason: String = format!("failed creating virtual memory snapshot (error={e:?})");
            error!("create_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        let mut file: File = File::create(kvm_filepath)?;

        let guest_state = self.guest.lock().await.save_state()?;
        Self::write_snapshot_component("guest", &guest_state, &mut file)?;

        let locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;

        let vcpu_state: VirtualProcessorState =
            self.vcpu.lock().await.save_state(&locked_inner.kvm)?;
        Self::write_snapshot_component("vcpu", &vcpu_state, &mut file)?;

        let irqchip_state: IrqChipState = locked_inner.irqchip.save_state(&locked_inner.vm)?;
        Self::write_snapshot_component("irqchip", &irqchip_state, &mut file)?;

        let timer_state: TimerState = locked_inner.timer.save_state(&locked_inner.vm)?;

        Self::write_snapshot_component("timer", &timer_state, &mut file)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes a snapshot component to the snapshot file.
    ///
    /// # Parameters
    ///
    /// - `component_name`: Name of the snapshot component.
    /// - `component`: Reference to the snapshot component.
    /// - `file`: Mutable reference to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    fn write_snapshot_component<T: ::serde::Serialize>(
        component_name: &str,
        component: &T,
        file: &mut File,
    ) -> Result<()> {
        match ::serde_cbor::to_vec(component) {
            Ok(buffer) => {
                if let Err(e) = file.write_all(&buffer) {
                    let reason: String =
                        format!("failed writing {component_name} snapshot (error={e:?})",);
                    error!("create_snapshot(): {reason}");
                    anyhow::bail!(reason)
                }
                trace!("wrote {} bytes to the snapshot file", buffer.len());
                Ok(())
            },
            Err(e) => {
                let reason: String =
                    format!("failed serializing {component_name} snapshot (error={e:?})",);
                error!("create_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Loads the virtual memory and the KVM state from files.
    ///
    /// # Parameters
    ///
    /// - `filepath`: Path to the executable file.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub async fn load_snapshot(&self, filepath: String) -> Result<()> {
        let (vmem_filepath, kvm_filepath) = Self::make_snapshot_paths(&filepath);

        if let Err(e) = self.vmem.lock().await.load_snapshot(&vmem_filepath) {
            let reason: String = format!("failed loading virtual memory snapshot (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        let mut file: File = match File::open(kvm_filepath) {
            Ok(f) => f,
            Err(e) => {
                let reason: String = format!("failed opening vcpu snapshot file (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };
        let state: VirtualProcessorState =
            match ::serde_cbor::from_reader::<VirtualProcessorState, &mut File>(&mut file) {
                Ok(state) => {
                    if let Err(e) = state.validate() {
                        let reason: String =
                            format!("decoded vcpu snapshot is invalid (error={e:?})");
                        error!("load_snapshot(): {reason}");
                        anyhow::bail!(reason)
                    } else {
                        state
                    }
                },
                Err(e) => {
                    let reason: String =
                        format!("failed decoding vcpu snapshot file (error={e:?})");
                    error!("load_snapshot(): {reason}");
                    anyhow::bail!(reason)
                },
            };

        if let Err(e) = self.vcpu.lock().await.load_state(&state) {
            let reason: String = format!("failed setting vcpu state (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        } else {
            Ok(())
        }
    }
}
