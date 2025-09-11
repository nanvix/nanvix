// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # MicroVM
//!
//! This module contains the front-end implementation of the MicroVM. Backend-end implementations
//! are provided by the [`kvm`](crate::kvm) modules.
//!

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::{
    orchestrator::{
        TIMEOUT_WARNING_INTERVAL_IN_MS,
        VcpuControlCommand,
        VcpuControlResponse,
    },
    vmm::microvm::kvm::{
        emulator::Emulator,
        partition::VirtualPartition,
        vcpu::{
            VirtualProcessor,
            VirtualProcessorExitContext,
            VirtualProcessorExitReason,
        },
        vmem::VirtualMemory,
    },
};

use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::libc::{
    SIGUSR1,
    c_int,
    sigaction,
    sigemptyset,
};
use ::std::{
    sync::{
        Arc,
        Mutex,
        MutexGuard,
        mpsc::{
            Receiver,
            Sender,
            TryRecvError,
        },
    },
    time::Instant,
};

pub const INTERRUPT_SIGNAL: c_int = SIGUSR1;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a MicroVM.
///
#[derive(Clone)]
pub struct MicroVm {
    // Virtual partition that hosts the virtual machine.
    _partition: Arc<Mutex<VirtualPartition>>,
    // Virtual memory of the virtual machine.
    vmem: Arc<Mutex<VirtualMemory>>,
    // Virtual processor of the virtual machine.
    vcpu: Arc<Mutex<VirtualProcessor>>,
    // Wraps fields that don't require individual `Arc<Mutex<_>>`s.
    handle: Arc<Mutex<InteriorMicroVmHandle>>,
}

///
/// # Description
///
/// An internal structure to the MicroVM that wraps its contents in `Arc<Mutex<_>>`. It allows
/// `MicroVm` to be clonable without wrapping each field in `Arc<Mutex<_>>`.
///
struct InteriorMicroVmHandle {
    // Emulator of the virtual machine.
    emulator: Emulator,
    // If present, initial RAM disk location and size.
    initrd: Option<(u64, usize)>,
    // Channel to receive commands from the VMM.
    control_rx: Receiver<VcpuControlCommand>,
    // Channel to send control responses to the VMM.
    control_tx: Sender<VcpuControlResponse>,
}

unsafe impl Send for InteriorMicroVmHandle {}
unsafe impl Sync for InteriorMicroVmHandle {}

//==================================================================================================
// Types
//==================================================================================================

pub type InputFn = dyn FnMut(&Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()>;

pub type OutputFn = dyn FnMut(&Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()>;

//==================================================================================================
// Implementations
//==================================================================================================

/// Signal handler for the vCPU thread. We install an empty handler to trigger an -EINTR.
extern "C" fn vcpu_thread_signal_handler(_: i32) {}

impl MicroVm {
    ///
    /// # Description
    ///
    /// Creates a MicroVM.
    ///
    /// # Parameters
    ///
    /// - `memory_size`: Size of the virtual memory of the virtual machine.
    /// - `input`: Input function used for emulating I/O port reads.
    /// - `output`: Output function used for emulating I/O port writes.
    /// - `control_rx`: Channel to receive commands from the VMM.
    /// - `control_tx`: Channel to send control responses to the VMM.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the MicroVM that was created. Otherwise, it
    /// returns an error.
    ///
    pub fn new(
        memory_size: usize,
        input: Box<InputFn>,
        output: Box<OutputFn>,
        control_rx: Receiver<VcpuControlCommand>,
        control_tx: Sender<VcpuControlResponse>,
    ) -> Result<Self> {
        trace!("new(): memory_size={memory_size}");
        crate::timer!("vm_creation");

        let partition: Arc<Mutex<VirtualPartition>> =
            Arc::new(Mutex::new(VirtualPartition::new()?));

        let vmem: Arc<Mutex<VirtualMemory>> =
            Arc::new(Mutex::new(VirtualMemory::new(partition.clone(), memory_size)?));

        let vcpu: Arc<Mutex<VirtualProcessor>> =
            Arc::new(Mutex::new(VirtualProcessor::new(partition.clone(), 0)?));

        let emulator: Emulator = Emulator::new(vmem.clone(), input, output)?;

        let state: Arc<Mutex<InteriorMicroVmHandle>> =
            Arc::new(Mutex::new(InteriorMicroVmHandle {
                emulator,
                initrd: None,
                control_rx,
                control_tx,
            }));

        Ok(Self {
            _partition: partition,
            vmem,
            vcpu,
            handle: state,
        })
    }

    ///
    /// # Description
    ///
    /// Loads a kernel into the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `kernel_filename`: Path to the kernel binary.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns the entry point of the program that was
    /// loaded into the virtual machine. Otherwise, it returns an error.
    ///
    pub fn load_kernel(&mut self, kernel_filename: &str) -> Result<u64> {
        trace!("load_kernel(): {kernel_filename}");
        crate::timer!("vm_load_kernel");
        let entry: u64 = self
            .vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .load_kernel(kernel_filename)?;
        Ok(entry)
    }

    ///
    /// # Description
    ///
    /// Loads an initial RAM disk into the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `initrd_filename`: Path to the initial RAM disk.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn load_initrd(&mut self, initrd_filename: &str) -> Result<()> {
        trace!("load_initrd(): {initrd_filename}");
        crate::timer!("vm_load_initrd");
        let initrd: (u64, usize) = self
            .vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .load_initrd(initrd_filename)?;
        self.handle
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .initrd = Some(initrd);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Writes a command line to the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `args`: Command line arguments to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_args(&mut self, args: &str) -> Result<()> {
        trace!("write_args(): args={args}");
        crate::timer!("vm_write_args");
        self.vmem
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {:?}", e))?
            .write_args(args)?;
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resets the virtual machine.
    ///
    /// # Parameters
    ///
    /// - `rip`: Entry point of the virtual machine.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn reset(&mut self, rip: u64) -> Result<()> {
        trace!("reset(): {rip:#010x}");
        crate::timer!("vm_reset");
        let rax: u64 = ::config::microvm::DEFAULT_BOOT_MAGIC as u64;

        // Check if initrd is too large.
        let nzeros: usize = ::config::microvm::DEFAULT_INITRD_BASE.trailing_zeros() as usize;
        let max_initrd_size: usize = (1 << 12) * ((1 << nzeros) - 1);
        let locked_state: MutexGuard<'_, InteriorMicroVmHandle> = self
            .handle
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;
        if let Some((_, initrd_size)) = locked_state.initrd {
            if initrd_size > max_initrd_size {
                return Err(anyhow::anyhow!(
                    "initrd is too large (initrd_size={initrd_size}, \
                     max_initrd_size={max_initrd_size:?})",
                ));
            }
        }

        // Retrieve initrd information.
        let (initrd_base, initrd_size): (u64, u64) = match locked_state.initrd {
            Some((base, size)) => (base, size as u64),
            None => (0, 0),
        };

        // Ensure that the initrd base and size are aligned to page size boundaries.
        assert_eq!(initrd_base as usize % PAGE_SIZE, 0, "initrd base is not aligned to page size");
        assert_eq!(initrd_size as usize % PAGE_SIZE, 0, "initrd size is not aligned to page size");

        // Encode initrd location and size:
        // - Lower bits encode the size in 4KB pages
        // - Higher bits encode the base address
        let rbx: u64 =
            (initrd_base & !((1 << nzeros) - 1)) | ((initrd_size >> 12) & ((1 << nzeros) - 1));

        self.vcpu
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .reset(rip, rax, rbx)
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
        crate::timer!("vm_run");

        // Install a signal handler in the virtual processor's thread.
        Self::install_signal_handler();

        loop {
            let exit_context: VirtualProcessorExitContext;
            // Lock scope.
            {
                let mut locked_vcpu: MutexGuard<'_, VirtualProcessor> = self
                    .vcpu
                    .lock()
                    .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;
                // Exit if the vCPU is no longer online.
                if !locked_vcpu.is_online() {
                    return Ok(locked_vcpu.exit_status());
                }
                exit_context = locked_vcpu.run()?;
            }

            // Parse exit reason.
            match exit_context.reason() {
                // The guest requested to access an I/O port.
                VirtualProcessorExitReason::PmioAccess => {
                    crate::timer!("vm_run_pmio_access");
                    let mut locked_state: MutexGuard<'_, InteriorMicroVmHandle> = self
                        .handle
                        .lock()
                        .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?;
                    if let Some(exit_status) =
                        locked_state.emulator.handle_pmio_access(exit_context)?
                    {
                        if exit_status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD {
                            self.vcpu
                                .lock()
                                .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                                .poweroff(exit_status);
                        }
                        // The Nanvix Daemon requested to pause, this means we need to suspend execution,
                        // but possibly resume it later.
                        else {
                            // This message changes the state from `PAUSE_REQUESTED` to `PAUSED`.
                            locked_state.control_tx.send(VcpuControlResponse::Paused)?;

                            let start: Instant = Instant::now();
                            let mut counter: usize = 1;
                            // TODO: exponential back-off timeout https://github.com/nanvix/nanvix/issues/943
                            loop {
                                match locked_state.control_rx.try_recv() {
                                    Ok(VcpuControlCommand::Resume) => break,
                                    // NOTE: Should we add an option for shutting down? Like so:
                                    // Ok(VcpuControlCommand::Shutdown) => self.vcpu.poweroff(0),
                                    Err(TryRecvError::Empty) => (),
                                    Err(TryRecvError::Disconnected) => {
                                        let reason: String = "the vmm has disconnected".to_string();
                                        error!("run(): {reason}");
                                        anyhow::bail!(reason)
                                    },
                                }

                                // NOTE: is it desirable to check for timeout in this case? If it is,
                                // should we use a larger constant, considering snapshots might take long?

                                // Log a warning and increment the counter every TIMEOUT_WARNING_INTERVAL_IN_MS ms.
                                let elapsed_time: usize = start.elapsed().as_millis() as usize;
                                if elapsed_time > TIMEOUT_WARNING_INTERVAL_IN_MS * counter {
                                    warn!(
                                        "{}ms have passed waiting for `ResumeMicroVm` message",
                                        TIMEOUT_WARNING_INTERVAL_IN_MS * counter
                                    );
                                    counter += 1;
                                }
                            }
                            trace!("MicroVM resumed");
                        }
                    }
                },

                // The guest requested to halt the virtual processor.
                VirtualProcessorExitReason::Halt => {
                    self.vcpu
                        .lock()
                        .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                        .poweroff(0);
                },

                // The guest was interrupted, this means we need to power-off.
                VirtualProcessorExitReason::Interrupted => {
                    self.vcpu
                        .lock()
                        .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
                        .poweroff(0);
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReason::Unknown => {
                    return Err(anyhow::anyhow!("unknown exit reason"));
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
    pub fn send_tid(&self, tid: u64) -> Result<()> {
        Ok(self
            .handle
            .lock()
            .map_err(|e| anyhow::anyhow!("failed to acquire lock {e:?}"))?
            .control_tx
            .send(VcpuControlResponse::Tid(tid))?)
    }
}
