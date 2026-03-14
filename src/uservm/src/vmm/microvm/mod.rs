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
pub mod ramfs;

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::{
    MicroVmArgs,
    guest::Guest,
    kvm::{
        KvmSnapshot,
        irqchip::IrqChip,
        timer::Timer,
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
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    ffi::OsStr,
    fs::File,
    io::Write,
    os::{
        fd::OwnedFd,
        unix::io::{
            AsRawFd,
            FromRawFd,
            RawFd,
        },
    },
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
pub use ramfs::RamFs;

//==================================================================================================
// IKC IRQ Notifier
//==================================================================================================

/// IRQ number used for IKC (inter-kernel communication) notifications.
const IKC_IRQ: u32 = 9;

///
/// # Description
///
/// Lightweight, lock-free notifier that injects an edge-triggered IKC interrupt (IRQ 9) into
/// the guest via a duplicated VM file descriptor.
///
/// The duplicated fd allows the memory thread to inject IRQs without contending on the
/// `tokio::Mutex` that protects the main `VmFd` used by the vCPU thread.
///
/// # Safety
///
/// `KVM_IRQ_LINE` is thread-safe — the KVM subsystem serialises concurrent ioctl calls
/// internally, so no user-space locking is required.
///
#[derive(Clone)]
pub struct IkcNotifier {
    /// Duplicated VM file descriptor for lock-free IRQ injection.
    vm_fd: Arc<OwnedFd>,
    /// Coalescing flag: when `true`, an IRQ has already been injected and the guest has not yet
    /// acknowledged it by consuming a credit, so further injections are redundant.
    pending: Arc<AtomicBool>,
}

impl IkcNotifier {
    /// Creates a new notifier by duplicating the given VM file descriptor.
    fn new(vm_fd: &VmFd, pending: Arc<AtomicBool>) -> Result<Self> {
        let raw_fd: RawFd = vm_fd.as_raw_fd();
        // SAFETY: `raw_fd` is a valid, open KVM VM file descriptor.
        // Use `F_DUPFD_CLOEXEC` so the duplicated fd is automatically close-on-exec,
        // preventing leaks across `exec()` if the process ever spawns children.
        let dup_fd: RawFd = unsafe { libc::fcntl(raw_fd, libc::F_DUPFD_CLOEXEC, 0) };
        if dup_fd < 0 {
            anyhow::bail!("failed to dup VM fd: {}", ::std::io::Error::last_os_error());
        }
        // SAFETY: `dup_fd` is a freshly duplicated, valid fd we now own.
        let owned: OwnedFd = unsafe { OwnedFd::from_raw_fd(dup_fd) };
        Ok(Self {
            vm_fd: Arc::new(owned),
            pending,
        })
    }

    /// Injects an edge-triggered IKC IRQ (assert then de-assert) into the guest.
    ///
    /// The call is coalesced: if a previous notification is still pending (the guest has not yet
    /// consumed a credit), the ioctl is skipped to avoid redundant host-kernel entries.
    pub fn notify(&self) -> Result<()> {
        // `swap(true)` returns the previous value.  If it was already `true` a notification is
        // outstanding and we can skip the expensive ioctl pair.
        if self.pending.swap(true, Ordering::AcqRel) {
            return Ok(());
        }

        let fd: RawFd = self.vm_fd.as_raw_fd();

        // Use `kvm_bindings::kvm_irq_level` and the `KVM_IRQ_LINE` ioctl number from
        // `vmm_sys_util` to avoid hardcoded magic constants and manual struct definitions.
        vmm_sys_util::ioctl_iow_nr!(KVM_IRQ_LINE, KVMIO, 0x61, kvm_bindings::kvm_irq_level);
        const KVMIO: u32 = 0xAE;

        let mut irq_level: kvm_bindings::kvm_irq_level = kvm_bindings::kvm_irq_level::default();
        irq_level.__bindgen_anon_1.irq = IKC_IRQ;
        irq_level.level = 1;

        // Assert IRQ.
        // SAFETY: fd is a valid KVM VM fd, irq_level is correctly initialised.
        let ret: i32 = unsafe { libc::ioctl(fd, KVM_IRQ_LINE(), &irq_level) };
        if ret != 0 {
            anyhow::bail!("KVM_IRQ_LINE assert failed: {}", ::std::io::Error::last_os_error());
        }

        // De-assert IRQ (edge trigger).
        irq_level.level = 0;
        // SAFETY: same as above.
        let ret: i32 = unsafe { libc::ioctl(fd, KVM_IRQ_LINE(), &irq_level) };
        if ret != 0 {
            anyhow::bail!("KVM_IRQ_LINE de-assert failed: {}", ::std::io::Error::last_os_error());
        }

        Ok(())
    }

    /// Clears the coalescing flag so that the next [`notify`](Self::notify) call will inject an
    /// IRQ. Call this from the vCPU thread after the guest has consumed at least one credit.
    pub fn acknowledge(&self) {
        self.pending.store(false, Ordering::Release);
    }
}

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
    /// Lock-free IKC interrupt notifier (duplicated VM fd).
    ikc_notifier: IkcNotifier,
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
    /// Path to the kernel file, used for deriving snapshot file paths.
    kernel_filename: String,
    /// When true, skip the next snapshot command (set after loading a snapshot to avoid
    /// re-triggering the snapshot when KVM re-executes the `out` instruction on restore).
    skip_next_snapshot: bool,
}

//==================================================================================================
// Types
//==================================================================================================

pub type StdinFn =
    dyn FnMut(&Arc<Mutex<Guest>>, &Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()> + Send;

pub type StdoutFn =
    dyn FnMut(&Arc<Mutex<VirtualMemory>>, &::sys::ipc::VmBusMessage) -> Result<()> + Send;

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
        let mut vmem: VirtualMemory =
            VirtualMemory::new(&mut kvm, &mut vm, ::config::kernel::MEMORY_SIZE)?;
        let guest: Arc<Mutex<Guest>> = if args.restoring_from_snapshot {
            // When restoring from a snapshot, skip kernel/initrd/ramfs loading and vCPU reset.
            // The snapshot restore will overwrite memory and CPU state.
            Arc::new(Mutex::new(Guest::default()))
        } else {
            let mut guest = Guest::default();

            guest.load_kernel(&mut vmem, &args.kernel_filename)?;
            args.initrd_filename
                .as_ref()
                .map(|initrd_filename| {
                    guest.load_initrd(&mut vmem, initrd_filename, args.initrd_args)
                })
                .transpose()?;

            let ramfs_region: Option<(usize, usize)> =
                if let Some(ramfs_filename) = args.ramfs_filename.as_deref() {
                    let initrd_end: usize = match guest.initrd_region() {
                        Some((base, size)) => match base.checked_add(size) {
                            Some(end) => end,
                            None => {
                                let reason: String = "initrd region overflowed while computing \
                                                      ramfs placement"
                                    .to_string();
                                error!("new(): {reason}");
                                anyhow::bail!(reason)
                            },
                        },
                        None => ::config::microvm::DEFAULT_INITRD_BASE,
                    };

                    let ramfs: RamFs = RamFs::open(Path::new(ramfs_filename))?;
                    let (ramfs_base, ramfs_size) =
                        ramfs.map_into_virtual_memory(&mut vmem, initrd_end)?;
                    vmem.attach_ramfs(ramfs);
                    Some((ramfs_base, ramfs_size))
                } else {
                    None
                };

            RamFs::write_registers(&mut vmem, ramfs_region)?;

            guest.reset(&mut vmem, &mut vcpu)?;

            // Setup KVM paravirtualized clock.
            //
            // NOTE: The pvclock page at DEFAULT_PVCLOCK_PAGE (GPA 0x1000) falls inside
            // the kernel ELF's `.zero` section (LOAD segment at GPA 0x0 with MemSiz
            // 0x8000). The ELF loader zero-fills this range when `load_kernel()` runs
            // above. Both `setup_pvclock()` (which causes KVM to populate the page)
            // and the boot-time write below must therefore execute **after** the ELF
            // has been loaded. This is the same pattern used by the microvm control
            // registers at GPA 0x0–0x10 (credits, pause-requested, ramfs).
            let pvclock_gpa: u64 = ::config::microvm::DEFAULT_PVCLOCK_PAGE as u64;
            vcpu.setup_pvclock(pvclock_gpa)?;

            // Write boot time (UTC nanoseconds since Unix epoch) to the pvclock page so
            // the guest can compute wall-clock time from the monotonic pvclock.
            let boot_time_ns: u64 = {
                let d: std::time::Duration = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let nanos_per_sec: u64 = u64::from(::sys::time::NANOSECONDS_PER_SECOND);
                d.as_secs() * nanos_per_sec + u64::from(d.subsec_nanos())
            };
            let boot_time_offset: u64 = (::config::microvm::DEFAULT_PVCLOCK_PAGE
                + ::config::microvm::PVCLOCK_BOOT_TIME_NS_OFFSET)
                as u64;
            vmem.write_bytes(boot_time_offset, &boot_time_ns.to_le_bytes())?;
            trace!("pvclock: boot_time_ns={boot_time_ns}, page_gpa={pvclock_gpa:#010x}");

            Arc::new(Mutex::new(guest))
        };

        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(vmem));

        let vcpu: Arc<Mutex<VirtualProcessor>> = Arc::new(Mutex::new(vcpu));

        let emulator: Emulator =
            Emulator::new(guest.clone(), vmem.clone(), args.input, args.output, args.stderr)?;

        // Create the IKC notifier *before* moving the VmFd into InteriorMicroVmHandle.
        let ikc_notifier: IkcNotifier = IkcNotifier::new(&vm, args.ikc_pending)?;

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
                kernel_filename: args.kernel_filename,
                skip_next_snapshot: false,
            })),
            ikc_notifier,
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
                sa_sigaction: vcpu_thread_signal_handler as *const () as usize,
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
                        if exit_status == ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD {
                            Handle::current().block_on(self.handle_snapshot())?;
                        } else if exit_status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD {
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

                // The guest was shutdown (triple fault).
                VirtualProcessorExitReasonRef::Shutdown => {
                    error!("run(): guest shutdown (triple fault)");
                    let exit_status: u16 = ErrorCode::IllegalByteSequence.into();
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReasonRef::Unknown => {
                    error!("run(): guest exited due to an unknown reason");
                    let exit_status: u16 = ErrorCode::IllegalByteSequence.into();
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
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
    /// Returns a cloneable, lock-free IKC interrupt notifier.
    ///
    /// The notifier uses a duplicated VM file descriptor so it can inject IRQs without
    /// contending on the main `Mutex<InteriorMicroVmHandle>` used by the vCPU thread.
    ///
    pub fn ikc_notifier(&self) -> IkcNotifier {
        self.ikc_notifier.clone()
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
    /// Handles a guest-initiated snapshot request from the VMM run loop.
    /// If the `skip_next_snapshot` flag is set (i.e., we are resuming from a restored snapshot),
    /// the request is silently skipped and the flag is cleared.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    async fn handle_snapshot(&self) -> Result<()> {
        // Scope the lock to avoid deadlock: `create_snapshot` re-acquires `self.inner`.
        let kernel_filename: String = {
            let mut locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;
            if locked_inner.skip_next_snapshot {
                trace!("handle_snapshot(): skipping snapshot (restored from snapshot)");
                locked_inner.skip_next_snapshot = false;
                return Ok(());
            }
            locked_inner.kernel_filename.clone()
        };
        match self.create_snapshot(kernel_filename).await {
            Ok(()) => {
                trace!("handle_snapshot(): snapshot created successfully");
                Ok(())
            },
            Err(error) => {
                error!("handle_snapshot(): failed to create snapshot: {error:?}");
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

        let locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;
        let kvm_snapshot: KvmSnapshot = KvmSnapshot::new(
            self.guest.lock().await.save_state()?,
            self.vcpu.lock().await.save_state(&locked_inner.kvm)?,
            locked_inner.irqchip.save_state(&locked_inner.vm)?,
            locked_inner.timer.save_state(&locked_inner.vm)?,
        );

        match ::serde_cbor::to_vec(&kvm_snapshot) {
            Ok(buffer) => {
                if let Err(e) = file.write_all(&buffer) {
                    let reason: String = format!("failed writing kvm snapshot (error={e:?})",);
                    error!("create_snapshot(): {reason}");
                    anyhow::bail!(reason)
                }
                trace!("wrote {} bytes to the snapshot file", buffer.len());
                Ok(())
            },
            Err(e) => {
                let reason: String = format!("failed serializing kvm snapshot (error={e:?})",);
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
                let reason: String = format!("failed opening kvm snapshot file (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let kvm_snapshot: KvmSnapshot =
            match ::serde_cbor::from_reader::<KvmSnapshot, &mut File>(&mut file) {
                Ok(snapshot) => snapshot,
                Err(e) => {
                    let reason: String = format!("failed decoding kvm snapshot file (error={e:?})");
                    error!("load_snapshot(): {reason}");
                    anyhow::bail!(reason)
                },
            };

        // Validate snapshot against host KVM capabilities before restoring.
        {
            let locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;
            if let Err(e) = kvm_snapshot.validate(&locked_inner.kvm) {
                let reason: String = format!("decoded kvm snapshot is invalid (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            }
        }

        // Load the snapshot.
        if let Err(e) = self
            .guest
            .lock()
            .await
            .restore_state(kvm_snapshot.get_guest_state())
        {
            let reason: String = format!("failed setting guest state (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        if let Err(e) = self
            .vcpu
            .lock()
            .await
            .load_state(kvm_snapshot.get_vcpu_state())
        {
            let reason: String = format!("failed setting vcpu state (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Destructure `locked_inner` to get the borrow checker to comply.
        let mut locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;
        let InteriorMicroVmHandle {
            vm, timer, irqchip, ..
        } = &mut *locked_inner;

        if let Err(e) = timer.restore_state(vm, kvm_snapshot.get_timer_state()) {
            let reason: String = format!("failed setting timer state (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        if let Err(e) = irqchip.restore_state(vm, kvm_snapshot.get_irqchip_state()) {
            let reason: String = format!("failed setting irqchip state (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        // Mark that the next snapshot command from the guest should be skipped,
        // since KVM will re-execute the `out` instruction that triggered the original snapshot.
        locked_inner.skip_next_snapshot = true;

        Ok(())
    }
}
