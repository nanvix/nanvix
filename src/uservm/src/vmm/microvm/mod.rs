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
pub mod ramfs;

cfg_if::cfg_if! {
    if #[cfg(feature = "whp")] {
        pub mod whp;
        pub use whp::*;
    } else {
        #[cfg(feature = "gdb")]
        pub mod gdb;
        pub mod kvm;
    }
}

//==================================================================================================
// KVM-specific implementation (Linux only)
//==================================================================================================

#[cfg(target_os = "linux")]
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
        VirtualProcessorDumpInfo,
        VirtualProcessorExitReasonRef,
    },
};
#[cfg(target_os = "linux")]
use ::anyhow::Result;
#[cfg(target_os = "linux")]
use ::kvm_ioctls::{
    Cap,
    Kvm,
    VmFd,
};
#[cfg(target_os = "linux")]
use ::libc::{
    SIGUSR1,
    c_int,
    sigaction,
    sigemptyset,
};
#[cfg(target_os = "linux")]
use ::log::{
    error,
    trace,
    warn,
};
#[cfg(target_os = "linux")]
use ::std::{
    cell::Cell,
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
#[cfg(target_os = "linux")]
use ::sys::error::ErrorCode;
#[cfg(target_os = "linux")]
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

#[cfg(all(target_os = "linux", feature = "profile-time"))]
use crate::perf::PerfTimings;
#[cfg(all(target_os = "linux", feature = "profile-time"))]
use ::std::time::Instant;

//==================================================================================================
// Re-Exports
//==================================================================================================

#[cfg(target_os = "linux")]
pub use kvm::vmem::VirtualMemory;
#[cfg(target_os = "linux")]
pub use ramfs::MultiRamFs;
#[cfg(target_os = "linux")]
pub use ramfs::RamFs;

//==================================================================================================
// IKC IRQ Notifier (Linux/KVM only)
//==================================================================================================

/// IRQ number used for IKC (inter-kernel communication) notifications.
#[cfg(target_os = "linux")]
const IKC_IRQ: u32 = 9;

/// Default profiling frequency in Hz for the guest profiler timer.
#[cfg(target_os = "linux")]
const DEFAULT_PROFILER_FREQ_HZ: u64 = 1000;

/// Minimum allowed profiler frequency (Hz) to avoid division by zero.
#[cfg(target_os = "linux")]
const MIN_PROFILER_FREQ_HZ: u64 = 1;

/// Maximum allowed profiler frequency (Hz) to avoid spin-loops.
#[cfg(target_os = "linux")]
const MAX_PROFILER_FREQ_HZ: u64 = 10_000;

/// Microseconds per second, used to compute the profiler timer period.
#[cfg(target_os = "linux")]
const MICROS_PER_SECOND: u64 = 1_000_000;

/// Environment variable controlling the profiling frequency (Hz).
#[cfg(target_os = "linux")]
const PROFILER_FREQ_ENV: &str = "NANVIX_PROFILER_FREQ_HZ";

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
#[cfg(target_os = "linux")]
#[derive(Clone)]
pub struct IkcNotifier {
    /// Duplicated VM file descriptor for lock-free IRQ injection.
    vm_fd: Arc<OwnedFd>,
    /// Coalescing flag: when `true`, an IRQ has already been injected and the guest has not yet
    /// acknowledged it by consuming a credit, so further injections are redundant.
    pending: Arc<AtomicBool>,
}

#[cfg(target_os = "linux")]
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
// Constants (Linux/KVM only)
//==================================================================================================

/// Signal used to interrupt the vCPU thread.
#[cfg(target_os = "linux")]
pub const INTERRUPT_SIGNAL: c_int = SIGUSR1;

/// Signal used for profiler timer interrupts. We use SIGUSR2 (not SIGUSR1)
/// because SIGUSR1 is already used by the orchestrator for shutdown, and its
/// handler sets the SHUTDOWN flag. The profiler needs a signal that merely
/// interrupts KVM_RUN with -EINTR without triggering shutdown.
#[cfg(target_os = "linux")]
pub const PROFILER_SIGNAL: c_int = libc::SIGUSR2;

//==================================================================================================
// Thread-Local Variables (Linux/KVM only)
//==================================================================================================

#[cfg(target_os = "linux")]
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

    /// Address of KVM's immediate-exit byte for the vCPU running on this thread.
    static KVM_IMMEDIATE_EXIT: Cell<*mut u8> = const { Cell::new(::core::ptr::null_mut()) };
}

//==================================================================================================
// Structures (Linux/KVM only)
//==================================================================================================

/// Clears the thread-local KVM immediate-exit pointer when the vCPU run loop exits.
#[cfg(target_os = "linux")]
struct KvmImmediateExitGuard;

#[cfg(target_os = "linux")]
impl KvmImmediateExitGuard {
    /// Registers KVM's immediate-exit byte for the current vCPU thread.
    ///
    /// # Safety
    ///
    /// `immediate_exit` must remain valid and writable until the returned guard is dropped.
    unsafe fn register(immediate_exit: *mut u8) -> Self {
        // SAFETY: The caller guarantees that `immediate_exit` points into the live vCPU mapping.
        unsafe { immediate_exit.write_volatile(0) };
        KVM_IMMEDIATE_EXIT.with(|slot| slot.set(immediate_exit));
        Self
    }
}

#[cfg(target_os = "linux")]
impl Drop for KvmImmediateExitGuard {
    fn drop(&mut self) {
        KVM_IMMEDIATE_EXIT.with(|slot| slot.set(::core::ptr::null_mut()));
    }
}

///
/// # Description
///
/// A structure that represents a VMM.
///
#[cfg(target_os = "linux")]
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
    /// Performance timings collector for fine-grained startup breakdown.
    #[cfg(feature = "profile-time")]
    perf_timings: PerfTimings,
    /// Optional GDB server TCP port (standalone mode only).
    #[cfg(feature = "gdb")]
    gdb_port: Option<u16>,
    /// Guest profiler handle (used by the run loop to record samples).
    guest_profiler:
        Option<std::sync::Arc<std::sync::Mutex<Vec<crate::guest_profiler::StackSample>>>>,
}

///
/// # Description
///
/// An internal structure to the VMM that wraps its contents in `Arc<Mutex<_>>`. It allows
/// `MicroVm` to be clonable without wrapping each field in `Arc<Mutex<_>>`.
///
#[cfg(target_os = "linux")]
pub(crate) struct InteriorMicroVmHandle {
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
    /// When true, the guest is entitled to take exactly one snapshot.
    /// Set from the `snapshot` kernel option; consumed on the first successful request.
    snapshot_allowed: bool,
}

//==================================================================================================
// Types (Linux/KVM only)
//==================================================================================================

#[cfg(target_os = "linux")]
pub type StdinFn =
    dyn FnMut(&Arc<Mutex<Guest>>, &Arc<Mutex<VirtualMemory>>, u32, usize) -> Result<()> + Send;

#[cfg(target_os = "linux")]
pub type StdoutFn =
    dyn FnMut(&Arc<Mutex<VirtualMemory>>, &::sys::ipc::VmBusMessage) -> Result<()> + Send;

#[cfg(target_os = "linux")]
pub type StderrFn = dyn Write + Send;

//==================================================================================================
// Implementations (Linux/KVM only)
//==================================================================================================

/// Signal handler for the vCPU thread. Sets the shutdown flag to stop re-entering KVM_RUN.
#[cfg(target_os = "linux")]
extern "C" fn vcpu_thread_signal_handler(_: i32) {
    SHUTDOWN.with(|shutdown| shutdown.store(true, Ordering::SeqCst));
    KVM_IMMEDIATE_EXIT.with(|slot| {
        let immediate_exit: *mut u8 = slot.get();
        if !immediate_exit.is_null() {
            // SAFETY: The pointer is registered while the vCPU mapping is live and is cleared
            // before that mapping can be dropped. The KVM API permits this write from a signal
            // handler to make a subsequent KVM_RUN return with EINTR.
            unsafe { immediate_exit.write_volatile(1) };
        }
    });
}

/// No-op signal handler for profiler timer. Only purpose is to interrupt
/// KVM_RUN with -EINTR so we can read guest registers for stack sampling.
/// Must NOT set SHUTDOWN — the VM continues running after sampling.
#[cfg(target_os = "linux")]
extern "C" fn profiler_signal_handler(_: i32) {
    // Intentionally empty: the signal itself causes KVM_RUN to return
    // with errno=EINTR, which surfaces as an Interrupted exit reason.
}

#[cfg(target_os = "linux")]
impl InteriorMicroVmHandle {
    /// Returns a mutable reference to the emulator.
    pub(crate) fn emulator_mut(&mut self) -> &mut Emulator {
        &mut self.emulator
    }
}

#[cfg(target_os = "linux")]
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

        #[cfg(feature = "profile-time")]
        let perf_timings: PerfTimings = args.perf_timings.clone();

        // Phase: KVM partition creation (KVM fd + VM fd + irqchip + timer).
        #[cfg(feature = "profile-time")]
        let partition_create_start: Instant = Instant::now();

        let mut kvm: Kvm = Kvm::new()?;
        if !kvm.check_extension(Cap::ImmediateExit) {
            let reason: String = "KVM_CAP_IMMEDIATE_EXIT is required".to_string();
            error!("new(): {reason}");
            anyhow::bail!(reason);
        }
        let mut vm: VmFd = kvm.create_vm()?;

        let irqchip: IrqChip = IrqChip::new(&mut kvm, &mut vm)?;
        let timer: Timer = Timer::new(&mut kvm, &mut vm)?;

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_partition_create(partition_create_start.elapsed().as_micros() as u64);

        #[cfg(feature = "profile-time")]
        let vcpu_create_start: Instant = Instant::now();

        let mut vcpu: VirtualProcessor = VirtualProcessor::new(&mut kvm, &mut vm, 0)?;

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_vcpu_create(vcpu_create_start.elapsed().as_micros() as u64);

        #[cfg(feature = "profile-time")]
        let vmem_create_start: Instant = Instant::now();

        let mut vmem: VirtualMemory =
            VirtualMemory::new(&mut kvm, &mut vm, ::config::kernel::MEMORY_SIZE)?;

        #[cfg(feature = "profile-time")]
        #[allow(clippy::cast_possible_truncation)]
        perf_timings.set_vmem_create(vmem_create_start.elapsed().as_micros() as u64);

        // Determine whether the snapshot kernel option is present.
        let snapshot_allowed: bool = args.kernel_args.as_deref().is_some_and(|kargs| {
            ::koptions::parse(kargs).contains(&::koptions::KernelOption::Snapshot)
        });

        let guest: Arc<Mutex<Guest>> = if args.restoring_from_snapshot {
            // When restoring from a snapshot, skip kernel/initrd/ramfs loading and vCPU reset.
            // The snapshot restore will overwrite memory and CPU state.
            Arc::new(Mutex::new(Guest::default()))
        } else {
            let mut guest = Guest::default();

            // Phase: Kernel loading.
            #[cfg(feature = "profile-time")]
            let kernel_load_start: Instant = Instant::now();

            guest.load_kernel(&mut vmem, &args.kernel_filename)?;

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_kernel_load(kernel_load_start.elapsed().as_micros() as u64);

            // Write kernel arguments to guest control registers. These registers reside in
            // the kernel ELF's `.zero` section, which `load_kernel()` zero-fills by default, so
            // this write must happen after it. (With the `nightly-performance-optimizations`
            // feature the loader skips that zeroing and relies on the freshly allocated guest
            // memory already being zero, but writing after `load_kernel()` remains correct.)
            if let Some(ref kargs) = args.kernel_args {
                Guest::write_kernel_args(&mut vmem, kargs)?;
            }

            // Phase: Initrd loading.
            #[cfg(feature = "profile-time")]
            let initrd_load_start: Instant = Instant::now();

            args.initrd_filename
                .as_ref()
                .map(|initrd_filename| {
                    guest.load_initrd(&mut vmem, initrd_filename, args.initrd_args)
                })
                .transpose()?;

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_initrd_load(initrd_load_start.elapsed().as_micros() as u64);

            // Phase: RamFS loading.
            #[cfg(feature = "profile-time")]
            let ramfs_load_start: Instant = Instant::now();

            let ramfs_region: Option<(usize, usize)> = {
                let initrd_end: usize = match guest.initrd_region() {
                    Some((base, size)) => match base.checked_add(size) {
                        Some(end) => end,
                        None => {
                            let reason: String = "initrd region overflowed while computing ramfs \
                                                  placement"
                                .to_string();
                            error!("new(): {reason}");
                            anyhow::bail!(reason)
                        },
                    },
                    None => ::config::microvm::DEFAULT_INITRD_BASE,
                };

                let loaded: ramfs::LoadedRamFs =
                    ramfs::load_ramfs(&mut vmem, initrd_end, args.ramfs_filename.as_deref(), &[])?;

                match loaded {
                    ramfs::LoadedRamFs::Single { ramfs, base, size } => {
                        vmem.attach_ramfs(ramfs);
                        Some((base, size))
                    },
                    ramfs::LoadedRamFs::None => None,
                }
            };

            RamFs::write_registers(&mut vmem, ramfs_region)?;

            // Write host TSC base frequency so the guest can use RDTSC-based LAPIC
            // timer calibration without requiring CPUID leaf 0x16.
            let tsc_freq_mhz: u32 = ::arch::cpu::cpuid::get_base_frequency_mhz();
            vmem.write_bytes(
                ::config::microvm::DEFAULT_MICROVM_CTRL_TSC_FREQ_MHZ as u64,
                &tsc_freq_mhz.to_le_bytes(),
            )?;
            trace!("ctrl: tsc_freq_mhz={tsc_freq_mhz}");

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_ramfs_load(ramfs_load_start.elapsed().as_micros() as u64);

            // Phase: vCPU reset and pvclock setup.
            #[cfg(feature = "profile-time")]
            let vcpu_reset_start: Instant = Instant::now();

            guest.reset(&mut vmem, &mut vcpu)?;

            // Setup KVM paravirtualized clock.
            //
            // NOTE: The pvclock page at DEFAULT_PVCLOCK_PAGE (GPA 0x1000) falls inside
            // the kernel ELF's `.zero` section (LOAD segment at GPA 0x0 with MemSiz
            // 0x8000), which `load_kernel()` zero-fills by default. `setup_pvclock()`
            // (which causes KVM to populate the page) and the boot-time write below must
            // therefore run after kernel loading, alongside the microvm control registers
            // at GPA 0x0–0x10 (credits, pause-requested, ramfs). (With the
            // `nightly-performance-optimizations` feature the loader skips that zeroing and
            // relies on the freshly allocated guest memory already being zero, but running
            // after `load_kernel()` remains correct.)
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

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_vcpu_reset(vcpu_reset_start.elapsed().as_micros() as u64);

            // Phase: EPT pre-population.
            // Pre-populate host pages for the kernel and initrd regions so that KVM's EPT fault
            // path only needs to install SLAT entries without host page faults.  This moves
            // page-fault costs from guest execution time to setup time.
            #[cfg(feature = "profile-time")]
            let ept_populate_start: Instant = Instant::now();

            vmem.populate_ept(&guest.ept_populate_ranges()?)?;

            #[cfg(feature = "profile-time")]
            #[allow(clippy::cast_possible_truncation)]
            perf_timings.set_ept_populate(ept_populate_start.elapsed().as_micros() as u64);

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
                snapshot_allowed,
            })),
            ikc_notifier,
            #[cfg(feature = "profile-time")]
            perf_timings,
            #[cfg(feature = "gdb")]
            gdb_port: args.gdb_port,
            guest_profiler: None,
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || self.run())
    }

    ///
    /// # Description
    ///
    /// Spawns a timer thread that periodically sends SIGUSR2 to the vCPU thread, interrupting
    /// KVM_RUN so the profiler can capture guest register state for stack sampling.
    ///
    /// # Parameters
    ///
    /// - `stop`: Atomic flag used to signal the timer thread to stop.
    /// - `vcpu_tid`: pthread ID of the vCPU thread to which SIGUSR2 will be sent.
    ///
    /// # Returns
    ///
    /// Returns a JoinHandle for the spawned timer thread.
    ///
    fn spawn_profiler_timer(
        stop: Arc<AtomicBool>,
        vcpu_tid: libc::pthread_t,
    ) -> std::thread::JoinHandle<()> {
        let freq_hz: u64 = std::env::var(PROFILER_FREQ_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_PROFILER_FREQ_HZ)
            .clamp(MIN_PROFILER_FREQ_HZ, MAX_PROFILER_FREQ_HZ);
        let period: std::time::Duration =
            std::time::Duration::from_micros(MICROS_PER_SECOND / freq_hz);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(period);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let ret: c_int = unsafe { libc::pthread_kill(vcpu_tid, PROFILER_SIGNAL) };
                if ret != 0 {
                    warn!("profiler: pthread_kill failed (errno={ret})");
                }
            }
        })
    }

    ///
    /// # Description
    ///
    /// Captures a guest profiler sample by reading vCPU registers and walking the frame-pointer
    /// chain through guest virtual memory.
    ///
    /// # Parameters
    ///
    /// - `profiler`: Shared vector of stack samples where the captured sample will be stored.
    /// - `vmem`: Handle to the guest virtual memory used to walk the frame-pointer chain.
    /// - `eip`: Instruction pointer of the guest at the time of the sample.
    /// - `ebp`: Base pointer of the guest at the time of the sample.
    /// - `cr3`: Page table base register of the guest at the time of the sample.
    ///
    fn capture_profiler_sample(
        profiler: &std::sync::Arc<std::sync::Mutex<Vec<crate::guest_profiler::StackSample>>>,
        vmem: &Arc<Mutex<VirtualMemory>>,
        eip: u32,
        ebp: u32,
        cr3: u32,
    ) {
        let vmem_guard: MutexGuard<'_, VirtualMemory> = vmem.blocking_lock();
        crate::guest_profiler::GuestProfiler::capture_sample(
            profiler,
            vmem_guard.get_raw_ptr(),
            vmem_guard.get_size(),
            eip,
            ebp,
            cr3,
        );
    }

    ///
    /// # Description
    ///
    /// Install signal handlers on the vCPU thread.
    ///
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
    /// Install the SIGUSR2 no-op handler for the profiler timer.
    ///
    /// sa_flags=0 (no SA_RESTART): we intentionally want SIGUSR2 to interrupt blocking syscalls
    /// (KVM_RUN ioctl) with -EINTR so the run loop can read guest registers for sampling.
    ///
    fn install_profiler_signal_handler() {
        let ret: c_int = unsafe {
            let profiler_action: sigaction = sigaction {
                sa_sigaction: profiler_signal_handler as *const () as usize,
                sa_mask: {
                    let mut set: libc::sigset_t = std::mem::zeroed();
                    sigemptyset(&mut set);
                    set
                },
                sa_flags: 0,
                sa_restorer: None,
            };
            sigaction(PROFILER_SIGNAL, &profiler_action, std::ptr::null_mut())
        };
        if ret != 0 {
            let errno: i32 = unsafe { *libc::__errno_location() };
            error!("error installing profiler signal handler (errno={errno:?})");
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

        let immediate_exit: *mut u8 = self.vcpu.blocking_lock().immediate_exit_ptr();
        // SAFETY: `immediate_exit` points into the vCPU mapping owned by `self`. The guard is
        // dropped before `self` and clears the thread-local pointer on every return path.
        let _immediate_exit_guard: KvmImmediateExitGuard =
            unsafe { KvmImmediateExitGuard::register(immediate_exit) };

        let profiling: bool = self.guest_profiler.is_some();

        // Install signal handlers in the virtual processor's thread.
        Self::install_signal_handler();
        if profiling {
            Self::install_profiler_signal_handler();
        }

        // Publish the vCPU thread ID only after its shutdown handler and immediate-exit pointer
        // are ready, so the orchestrator cannot race an early shutdown request with setup.
        let pthread_id: libc::pthread_t = unsafe { libc::pthread_self() };
        Handle::current().block_on(self.send_tid(pthread_id))?;

        // When GDB server is enabled, delegate to the GDB event loop instead of the normal loop.
        #[cfg(feature = "gdb")]
        if let Some(port) = self.gdb_port {
            let exit_status = gdb::run_gdb_server(
                port,
                self.vcpu.clone(),
                self.vmem.clone(),
                self.inner.clone(),
            )?;
            Handle::current().block_on(self.handle_shutdown(exit_status));
            return Ok(exit_status);
        }

        // Accumulate guest execution time (inside KVM_RUN).
        #[cfg(feature = "profile-time")]
        let mut guest_time_acc_us: u64 = 0;
        #[cfg(feature = "profile-time")]
        let loop_start: Instant = Instant::now();

        // Guest profiler: start a timer thread that sends SIGUSR2 to
        // interrupt KVM_RUN periodically for stack sampling.
        let profiler_stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let profiler_thread: Option<std::thread::JoinHandle<()>> = if profiling {
            let vcpu_tid: libc::pthread_t = unsafe { libc::pthread_self() };
            Some(Self::spawn_profiler_timer(profiler_stop.clone(), vcpu_tid))
        } else {
            None
        };

        let result = loop {
            // Check shutdown flag before entering KVM_RUN, and blocking indefinitely.
            if SHUTDOWN.with(|shutdown| shutdown.load(Ordering::SeqCst)) {
                let exit_status: u16 = 0;
                Handle::current().block_on(self.handle_shutdown(exit_status));
                break Ok(exit_status);
            }

            let (exit_context, profile_regs) = {
                let mut locked_vcpu: MutexGuard<'_, VirtualProcessor> = self.vcpu.blocking_lock();
                // Exit if the vCPU is no longer online.
                if !locked_vcpu.is_online() {
                    break Ok(locked_vcpu.exit_status());
                }
                #[cfg(feature = "profile-time")]
                let run_start: Instant = Instant::now();

                let ctx = locked_vcpu.run();

                #[cfg(feature = "profile-time")]
                {
                    #![allow(clippy::cast_possible_truncation)]
                    guest_time_acc_us += run_start.elapsed().as_micros() as u64;
                }

                // Guest profiler: on Interrupted exits (from our SIGUSR2 timer), read guest
                // registers for stack sampling.  Both get_regs and get_sregs must succeed for a
                // valid sample.  Truncate 64-bit registers to 32-bit: the Nanvix guest runs in
                // 32-bit protected mode, so only the low 32 bits are meaningful.
                #[allow(clippy::cast_possible_truncation)]
                let regs: Option<(u32, u32, u32)> = if self.guest_profiler.is_some()
                    && matches!(ctx.reason_ref(), VirtualProcessorExitReasonRef::Interrupted)
                {
                    locked_vcpu.get_regs().ok().and_then(|r| {
                        locked_vcpu
                            .get_sregs()
                            .ok()
                            .map(|s| (r.rip as u32, r.rbp as u32, s.cr3 as u32))
                    })
                } else {
                    None
                };

                (ctx, regs)
            };

            // Guest profiler: capture sample after vcpu lock is released.
            if let (Some(profiler), Some((eip, ebp, cr3))) = (&self.guest_profiler, profile_regs) {
                Self::capture_profiler_sample(profiler, &self.vmem, eip, ebp, cr3);
            }

            // Parse exit reason.
            match exit_context.reason_ref() {
                // The guest requested to access an I/O port.
                VirtualProcessorExitReasonRef::PmioAccess(access) => {
                    let exit_status = self
                        .inner
                        .blocking_lock()
                        .emulator_mut()
                        .handle_pmio_access(access)?;
                    if let Some(exit_status) = exit_status {
                        if exit_status == ::config::microvm::DEFAULT_VMM_BOOT_COMPLETE_CMD {
                            // Kernel finished booting; no-op on KVM backend.
                        } else if exit_status == ::config::microvm::DEFAULT_VMM_SNAPSHOT_CMD {
                            // Guest requested a snapshot via the `snapshot` kernel option. This is
                            // a one-shot "save and exit" flow: once the snapshot files are
                            // durable on disk, shut the VM down with exit code 0 so the standalone
                            // daemon returns to its caller instead of running the guest on (which
                            // would otherwise block forever on stdin).
                            //
                            // `handle_snapshot()` returns `false` for the OUT that KVM re-executes
                            // immediately after a restore (absorbed via `skip_next_snapshot`); in
                            // that case keep running the restored guest.
                            let took_snapshot: bool =
                                Handle::current().block_on(self.handle_snapshot())?;
                            if took_snapshot {
                                let exit_status: u16 = 0;
                                Handle::current().block_on(self.handle_shutdown(exit_status));
                                break Ok(exit_status);
                            }
                        } else if exit_status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD {
                            Handle::current().block_on(self.handle_shutdown(exit_status));

                            break Ok(exit_status);
                        } else {
                            Handle::current().block_on(self.handle_pause())?;
                            trace!("VMM resumed");
                        }
                    }
                },

                // The guest was halted or interrupted.
                // When the profiler is active, Interrupted exits are from our SIGUSR2 timer — just
                // continue the loop (samples were already captured above). Without the profiler,
                // Interrupted means the orchestrator requested shutdown via SIGUSR1.
                VirtualProcessorExitReasonRef::Halt
                | VirtualProcessorExitReasonRef::Interrupted => {
                    if self.guest_profiler.is_some()
                        && matches!(
                            exit_context.reason_ref(),
                            VirtualProcessorExitReasonRef::Interrupted
                        )
                        && !SHUTDOWN.with(|s| s.load(Ordering::SeqCst))
                    {
                        // Profiler-induced interrupt: continue running.
                        continue;
                    }
                    let exit_status: u16 = 0;
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },

                // The guest was shutdown (triple fault).
                VirtualProcessorExitReasonRef::Shutdown => {
                    error!("run(): guest shutdown (triple fault)");
                    self.dump_vm_info();
                    let exit_status: u16 = ErrorCode::IllegalByteSequence.into();
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },

                // Debug event without GDB server — treat as unknown.
                VirtualProcessorExitReasonRef::DebugEvent => {
                    warn!("run(): debug exit without GDB server enabled");
                    self.dump_vm_info();
                    let exit_status: u16 = ErrorCode::IllegalByteSequence.into();
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReasonRef::Unknown => {
                    error!("run(): guest exited due to an unknown reason");
                    self.dump_vm_info();
                    let exit_status: u16 = ErrorCode::IllegalByteSequence.into();
                    Handle::current().block_on(self.handle_shutdown(exit_status));
                    break Ok(exit_status);
                },
            }
        };

        // Stop profiler timer. Relaxed ordering is sufficient here because join() provides the
        // necessary synchronization barrier, and a stray SIGUSR2 after the flag is set is harmless
        // (the no-op handler runs, and the SHUTDOWN check prevents re-entering the loop). The timer
        // thread is joined while still inside run() (the vCPU thread), so vcpu_tid remains valid
        // until after join() completes.
        profiler_stop.store(true, Ordering::Relaxed);
        if let Some(t) = profiler_thread
            && let Err(e) = t.join()
        {
            warn!("profiler timer thread panicked: {:?}", e);
        }

        // Record guest vs exit-handling time breakdown.
        #[cfg(feature = "profile-time")]
        {
            #[allow(clippy::cast_possible_truncation)]
            let loop_total_us: u64 = loop_start.elapsed().as_micros() as u64;
            self.perf_timings.set_guest_exec(guest_time_acc_us);
            self.perf_timings
                .set_exit_handling(loop_total_us.saturating_sub(guest_time_acc_us));
        }

        result
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
    /// #Description
    ///
    /// Enables guest stack profiling. Returns the `GuestProfiler` whose sample buffer is shared
    /// with the run loop. The caller drains it after VM exit to produce folded stacks.
    ///
    /// On KVM, the run loop starts a timer thread that sends SIGUSR2 to interrupt KVM_RUN at the
    /// configured frequency. On each Interrupted exit, guest registers (EIP/EBP/CR3) are read and a
    /// frame-pointer walk captures the guest call stack.
    ///
    /// # Returns
    ///
    /// Returns a `GuestProfiler` instance that can be used to collect guest stack samples.
    ///
    pub fn enable_guest_profiler(&mut self) -> crate::guest_profiler::GuestProfiler {
        let guest_profiler = crate::guest_profiler::GuestProfiler::new(
            crate::guest_profiler::DEFAULT_SAMPLE_CAPACITY,
        );
        self.guest_profiler = Some(guest_profiler.handle());
        guest_profiler
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
                // Don't bail as we are shutting down anyway. The orchestrator will retry the
                // backend shutdown request if its wait times out.
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
    ///
    /// Background: after a restore, KVM re-executes the `out` instruction that originally
    /// triggered the snapshot (its RIP was saved pointing at the OUT). `load_snapshot()` sets
    /// `skip_next_snapshot = true` so that re-emitted OUT is swallowed here instead of producing
    /// a second snapshot file.
    ///
    /// # Returns
    ///
    /// On success, returns `Ok(true)` when a snapshot was actually written to disk (caller should
    /// treat this as a "save and exit" event), and `Ok(false)` when the snapshot OUT was silently
    /// absorbed via `skip_next_snapshot` (caller should continue running the restored guest).
    /// Otherwise, returns an error.
    ///
    async fn handle_snapshot(&self) -> Result<bool> {
        // Locking: the lock is scoped tightly because `create_snapshot()` re-acquires
        // `self.inner`. No concurrent snapshot requests can race because snapshot is triggered by
        // a single vCPU VMEXIT processed sequentially on the VMM run loop.
        let kernel_filename: String = {
            let mut locked_inner: MutexGuard<'_, InteriorMicroVmHandle> = self.inner.lock().await;
            if locked_inner.skip_next_snapshot {
                trace!("handle_snapshot(): skipping snapshot (restored from snapshot)");
                locked_inner.skip_next_snapshot = false;
                return Ok(false);
            }
            if !locked_inner.snapshot_allowed {
                error!("handle_snapshot(): snapshot refused (not enabled or already consumed)");
                anyhow::bail!(
                    "snapshot refused: not enabled via kernel option or already consumed"
                );
            }
            locked_inner.kernel_filename.clone()
        };
        match self.create_snapshot(kernel_filename).await {
            Ok(()) => {
                // Consume the one-shot permission only after a successful snapshot.
                self.inner.lock().await.snapshot_allowed = false;
                trace!("handle_snapshot(): snapshot created successfully");
                Ok(true)
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
        #[cfg(feature = "profile-time")]
        let snapshot_creation_start: Instant = Instant::now();

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
                #[cfg(feature = "profile-time")]
                {
                    #![allow(clippy::cast_possible_truncation)]
                    let elapsed_us: u64 = snapshot_creation_start.elapsed().as_micros() as u64;
                    self.perf_timings.set_snapshot_creation(elapsed_us);
                }
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

    /// Requests shutdown of the KVM vCPU thread.
    pub fn request_shutdown(&self, vcpu_tid: u64) {
        let pthread_id: libc::pthread_t = vcpu_tid as libc::pthread_t;
        let result: c_int = unsafe { libc::pthread_kill(pthread_id, INTERRUPT_SIGNAL) };
        if result != 0 {
            warn!("request_shutdown(): failed to signal vCPU thread (error={result})");
        }
    }

    //==============================================================================================
    // Diagnostic Dump Helpers
    //==============================================================================================

    /// Number of bytes to dump around RIP (instruction pointer).
    const CODE_DUMP_RADIUS: u64 = 32;
    /// Number of bytes to dump around RSP (stack pointer).
    const STACK_DUMP_RADIUS: u64 = 64;
    /// Maximum number of stack frames to walk via the RBP chain.
    const MAX_STACK_FRAMES: usize = 32;

    ///
    /// # Description
    ///
    /// Dumps the virtual machine state for diagnostic purposes.
    ///
    /// This method reads the vCPU registers and guest memory, then logs:
    /// - General-purpose registers, control registers, segment registers, and descriptor tables.
    /// - Code bytes around RIP (instruction vicinity).
    /// - Stack bytes around RSP (stack vicinity).
    /// - Stack trace by walking the RBP frame-pointer chain.
    ///
    /// All memory reads use guest physical addresses. This is correct because the Nanvix kernel
    /// identity-maps its virtual address space.
    ///
    /// Failures to read registers or memory are logged but do not propagate — this method
    /// is best-effort diagnostics.
    ///
    fn dump_vm_info(&self) {
        // Read vCPU registers (narrow scope releases the lock before acquiring vmem).
        let info: VirtualProcessorDumpInfo = {
            let vcpu: MutexGuard<'_, VirtualProcessor> = self.vcpu.blocking_lock();
            match vcpu.get_dump_info() {
                Ok(i) => i,
                Err(e) => {
                    error!("dump_vm_info(): failed to read registers (error={e:?})");
                    return;
                },
            }
        };

        // Dump register state.
        Self::dump_registers(&info);

        // Dump memory context (narrow scope releases the lock at the end).
        {
            let vmem: MutexGuard<'_, VirtualMemory> = self.vmem.blocking_lock();
            let mem_size: u64 = vmem.get_size() as u64;

            // Dump code bytes around RIP.
            Self::dump_region(&vmem, mem_size, "Code", info.rip, Self::CODE_DUMP_RADIUS);

            // Dump stack bytes around RSP.
            Self::dump_region(&vmem, mem_size, "Stack", info.rsp, Self::STACK_DUMP_RADIUS);

            // Walk the RBP frame-pointer chain to produce a stack trace.
            Self::dump_stack_trace(&vmem, mem_size, info.rip, info.rbp);
        }
    }

    ///
    /// # Description
    ///
    /// Dumps general-purpose registers, control registers, segment registers, and descriptor
    /// table pointers from a hypervisor-independent register snapshot.
    ///
    /// # Parameters
    ///
    /// - `info`: Register snapshot to dump.
    ///
    fn dump_registers(info: &VirtualProcessorDumpInfo) {
        error!("=== General Purpose Registers ===");
        error!("  RIP={:#018x}  RFLAGS={:#018x}", info.rip, info.rflags);
        error!(
            "  RAX={:#018x}  RBX={:#018x}  RCX={:#018x}  RDX={:#018x}",
            info.rax, info.rbx, info.rcx, info.rdx
        );
        error!(
            "  RSI={:#018x}  RDI={:#018x}  RSP={:#018x}  RBP={:#018x}",
            info.rsi, info.rdi, info.rsp, info.rbp
        );
        error!(
            "  R8 ={:#018x}  R9 ={:#018x}  R10={:#018x}  R11={:#018x}",
            info.r8, info.r9, info.r10, info.r11
        );
        error!(
            "  R12={:#018x}  R13={:#018x}  R14={:#018x}  R15={:#018x}",
            info.r12, info.r13, info.r14, info.r15
        );
        error!("=== Control Registers ===");
        error!(
            "  CR0={:#018x}  CR2={:#018x}  CR3={:#018x}  CR4={:#018x}  CR8={:#018x}  EFER={:#018x}",
            info.cr0, info.cr2, info.cr3, info.cr4, info.cr8, info.efer
        );
        error!("=== Segment Registers ===");
        error!(
            "  CS:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.cs.selector, info.cs.base, info.cs.limit
        );
        error!(
            "  DS:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.ds.selector, info.ds.base, info.ds.limit
        );
        error!(
            "  SS:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.ss.selector, info.ss.base, info.ss.limit
        );
        error!(
            "  ES:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.es.selector, info.es.base, info.es.limit
        );
        error!(
            "  FS:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.fs.selector, info.fs.base, info.fs.limit
        );
        error!(
            "  GS:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.gs.selector, info.gs.base, info.gs.limit
        );
        error!("=== Descriptor Tables ===");
        error!("  GDT: base={:#018x}  limit={:#06x}", info.gdt.base, info.gdt.limit);
        error!("  IDT: base={:#018x}  limit={:#06x}", info.idt.base, info.idt.limit);
        error!(
            "  TR:  selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.tr.selector, info.tr.base, info.tr.limit
        );
        error!(
            "  LDT: selector={:#06x}  base={:#018x}  limit={:#010x}",
            info.ldt.selector, info.ldt.base, info.ldt.limit
        );
    }

    ///
    /// # Description
    ///
    /// Dumps a region of guest memory around a given address.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Handle to the guest virtual memory.
    /// - `mem_size`: Total size of guest memory in bytes.
    /// - `label`: Human-readable label for the region (e.g., "Code", "Stack").
    /// - `center`: Address around which to dump.
    /// - `radius`: Number of bytes to dump before and after `center`.
    ///
    fn dump_region(vmem: &VirtualMemory, mem_size: u64, label: &str, center: u64, radius: u64) {
        let start: u64 = center.saturating_sub(radius);
        let end: u64 = center.saturating_add(radius).min(mem_size);

        if start >= mem_size || start >= end {
            error!("=== {label} Dump (addr={center:#018x}) ===");
            error!("  address outside guest memory");
            return;
        }

        let len: usize = match usize::try_from(end - start) {
            Ok(v) => v,
            Err(_) => {
                error!("=== {label} Dump (addr={center:#018x}) ===");
                error!("  region too large to dump");
                return;
            },
        };
        let mut buf: Vec<u8> = vec![0u8; len];
        if let Err(e) = vmem.read_bytes(start, &mut buf) {
            error!("=== {label} Dump (addr={center:#018x}) ===");
            error!("  failed to read memory (error={e:?})");
            return;
        }

        error!("=== {label} Dump ({center:#018x}, {start:#018x}..{end:#018x}) ===");
        for (i, chunk) in buf.chunks(16).enumerate() {
            let addr: u64 = start + (i as u64) * 16;
            let hex: String = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            error!("  {addr:#018x}: {hex}");
        }
    }

    ///
    /// # Description
    ///
    /// Walks the RBP frame-pointer chain and logs a stack trace.
    ///
    /// Each x86-64 stack frame (with frame pointers) stores the caller's RBP at [RBP] and the
    /// return address at [RBP+8]. The walk terminates when RBP is zero, points outside guest
    /// memory, or the maximum frame depth is reached.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Handle to the guest virtual memory.
    /// - `mem_size`: Total size of guest memory in bytes.
    /// - `rip`: Current instruction pointer (frame 0 return address).
    /// - `rbp`: Current base pointer (start of the frame chain).
    ///
    fn dump_stack_trace(vmem: &VirtualMemory, mem_size: u64, rip: u64, rbp: u64) {
        error!("=== Stack Trace ===");
        error!("  #{:<3} RIP={rip:#018x}", 0);

        let mut current_rbp: u64 = rbp;

        for frame in 1..Self::MAX_STACK_FRAMES {
            // Each frame requires 16 bytes: saved_rbp (8) + return_addr (8).
            if current_rbp == 0 || current_rbp.checked_add(16).is_none_or(|end| end > mem_size) {
                break;
            }

            let mut frame_data: [u8; 16] = [0u8; 16];
            if vmem.read_bytes(current_rbp, &mut frame_data).is_err() {
                error!("  #{frame:<3} <unreadable frame at RBP={current_rbp:#018x}>");
                break;
            }

            let saved_rbp: u64 = u64::from_le_bytes([
                frame_data[0],
                frame_data[1],
                frame_data[2],
                frame_data[3],
                frame_data[4],
                frame_data[5],
                frame_data[6],
                frame_data[7],
            ]);
            let ret_addr: u64 = u64::from_le_bytes([
                frame_data[8],
                frame_data[9],
                frame_data[10],
                frame_data[11],
                frame_data[12],
                frame_data[13],
                frame_data[14],
                frame_data[15],
            ]);

            error!("  #{frame:<3} RIP={ret_addr:#018x}  (RBP={current_rbp:#018x})");

            // Detect cycles or backward frame-pointer movement.
            if saved_rbp == 0 || saved_rbp <= current_rbp {
                break;
            }

            current_rbp = saved_rbp;
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn shutdown_signal_sets_kvm_immediate_exit() {
        let mut immediate_exit: u8 = 7;
        let immediate_exit_ptr: *mut u8 = ::core::ptr::addr_of_mut!(immediate_exit);
        // SAFETY: `immediate_exit` remains live until after the guard is dropped.
        let guard: KvmImmediateExitGuard =
            unsafe { KvmImmediateExitGuard::register(immediate_exit_ptr) };

        assert_eq!(immediate_exit, 0);
        SHUTDOWN.with(|shutdown| shutdown.store(false, Ordering::SeqCst));

        vcpu_thread_signal_handler(INTERRUPT_SIGNAL);

        assert!(SHUTDOWN.with(|shutdown| shutdown.load(Ordering::SeqCst)));
        assert_eq!(immediate_exit, 1);

        drop(guard);
        assert!(KVM_IMMEDIATE_EXIT.with(|slot| slot.get().is_null()));
        SHUTDOWN.with(|shutdown| shutdown.store(false, Ordering::SeqCst));
    }
}
