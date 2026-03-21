// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::module_inception)]
// The WHP backend interfaces with the Windows Hypervisor Platform API, which
// uses u32/u16 parameters extensively. These casts are intentional and safe.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::collapsible_match)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod emulator;
pub mod guest;
pub mod lapic;
pub mod partition;
pub mod ramfs;
pub mod timer;
pub mod vcpu;
pub mod vmem;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    orchestrator::{
        VcpuControlCommand,
        VcpuControlResponse,
    },
    vmm::{
        MicroVmArgs,
        emulator::Emulator,
        guest::Guest,
        whp::vcpu::{
            VirtualProcessor,
            VirtualProcessorExitReasonRef,
        },
    },
};
use ::anyhow::Result;
use ::log::{
    error,
    trace,
    warn,
};
use ::std::{
    io::Write,
    path::Path,
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

pub use vmem::VirtualMemory;

// Re-exports
//==================================================================================================

pub use crate::vmm::whp::vcpu::exit::{
    PmioAccess,
    PmioWidth,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Signal used to interrupt the vCPU thread (unused on Windows, but required by orchestrator).
pub const INTERRUPT_SIGNAL: i32 = 0;

/// Signal used to kill the vCPU thread (unused on Windows, but required by orchestrator).
pub const KILL_SIGNAL: i32 = 0;

//==================================================================================================
// IKC Notifier
//==================================================================================================

/// Notifier that signals the VMM loop to inject an IKC interrupt after the host writes credits.
///
/// With LAPIC emulation disabled, the VMM loop polls this flag on each
/// iteration (including Halt exits). No concurrent WHP API calls are
/// made — `notify()` only sets an atomic flag.
#[derive(Clone)]
pub struct IkcNotifier {
    pending: Arc<AtomicBool>,
    /// Set to true after the VMM has shut down.
    shutdown: Arc<AtomicBool>,
}

impl IkcNotifier {
    fn new() -> Self {
        Self {
            pending: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signals that an IKC message is available.
    ///
    /// Sets an atomic flag that the VMM loop checks on each iteration.
    /// No WHP API calls are made — the VMM loop picks up the pending
    /// flag on the next HLT or PMIO exit.
    pub fn notify(&self) -> Result<()> {
        if self.shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        self.pending.store(true, Ordering::Release);
        Ok(())
    }

    /// Returns true if an IKC notification is pending, and clears the flag.
    fn take_pending(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }

    /// Marks the notifier as shut down, preventing future `notify()` calls from invoking
    /// WHP APIs.
    fn mark_shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

//==================================================================================================
// Thread-Local Variables
//==================================================================================================

thread_local! {
    /// Shutdown flag, set to true when the vCPU thread should stop running.
    static SHUTDOWN: AtomicBool = const { AtomicBool::new(false) };
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents a VMM backed by Windows Hypervisor Platform (WHP).
///
#[derive(Clone)]
pub struct Vmm {
    /// Guest of the virtual machine.
    guest: Arc<Mutex<Guest>>,
    /// Virtual memory of the virtual machine.
    vmem: Arc<Mutex<VirtualMemory>>,
    /// Virtual processor of the virtual machine.
    vcpu: Arc<Mutex<VirtualProcessor>>,
    /// Host-side timer for periodic interrupt injection.
    timer: Arc<std::sync::Mutex<timer::Timer>>,
    /// IKC notifier for waking the guest when credits are added.
    ikc_notifier: IkcNotifier,
    /// Partition handle (for future interrupt injection use).
    #[allow(dead_code)]
    partition_handle: windows::Win32::System::Hypervisor::WHV_PARTITION_HANDLE,
    /// Wraps fields that don't require individual `Arc<Mutex<_>>`s.
    inner: Arc<Mutex<InteriorWhpHandle>>,
}

///
/// # Description
///
/// An internal structure to the VMM that wraps its contents in `Arc<Mutex<_>>`. It allows
/// the VMM to be clonable without wrapping each field in `Arc<Mutex<_>>`.
///
struct InteriorWhpHandle {
    /// WHP partition handle (must outlive vCPU and vmem).
    _partition: partition::WhpPartition,
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

pub type StdoutFn =
    dyn FnMut(&Arc<Mutex<VirtualMemory>>, &::sys::ipc::VmBusMessage) -> Result<()> + Send;

pub type StderrFn = dyn Write + Send;

//==================================================================================================
// Implementations
//==================================================================================================

impl Vmm {
    ///
    /// # Description
    ///
    /// Creates a VMM backed by Windows Hypervisor Platform.
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

        let partition: partition::WhpPartition = partition::WhpPartition::new()?;
        let mut vmem: VirtualMemory =
            VirtualMemory::new(&partition, ::config::kernel::MEMORY_SIZE)?;
        let mut vcpu: VirtualProcessor = VirtualProcessor::new(&partition, 0)?;

        let guest: Arc<Mutex<Guest>> = if args.restoring_from_snapshot {
            Arc::new(Mutex::new(Guest::default()))
        } else {
            let mut guest: Guest = Guest::default();

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

                    let ramfs: ramfs::RamFs = ramfs::RamFs::open(Path::new(ramfs_filename))?;
                    let (ramfs_base, ramfs_size) =
                        ramfs.load_into_virtual_memory(&mut vmem, initrd_end)?;
                    Some((ramfs_base, ramfs_size))
                } else {
                    None
                };

            ramfs::RamFs::write_registers(&mut vmem, ramfs_region)?;

            guest.reset(&mut vmem, &mut vcpu)?;

            // Populate the pvclock page so the kernel uses TSC-based time instead
            // of PIT tick counting. This must run AFTER load_kernel() because the
            // ELF loader zero-fills the page at DEFAULT_PVCLOCK_PAGE.
            Self::setup_pvclock(&mut vmem)?;

            Arc::new(Mutex::new(guest))
        };

        let partition_handle = partition.handle();
        let vmem: Arc<Mutex<VirtualMemory>> = Arc::new(Mutex::new(vmem));
        let vcpu: Arc<Mutex<VirtualProcessor>> = Arc::new(Mutex::new(vcpu));
        let timer: Arc<std::sync::Mutex<timer::Timer>> =
            Arc::new(std::sync::Mutex::new(timer::Timer::new(partition_handle)));

        let ikc_notifier = IkcNotifier::new();

        let emulator: Emulator =
            Emulator::new(guest.clone(), vmem.clone(), args.input, args.output, args.stderr)?;

        Ok(Self {
            guest,
            vmem,
            vcpu,
            timer,
            ikc_notifier,
            partition_handle,
            inner: Arc::new(Mutex::new(InteriorWhpHandle {
                _partition: partition,
                emulator,
                control_rx: args.control_rx,
                control_tx: args.control_tx,
            })),
        })
    }

    pub fn spawn(mut self) -> tokio::task::JoinHandle<Result<u16>> {
        task::spawn_blocking(move || {
            let thread_id: u64 =
                unsafe { windows::Win32::System::Threading::GetCurrentThreadId() as u64 };
            Handle::current().block_on(self.send_tid(thread_id))?;

            warn!("VMM spawn_blocking: calling run()");
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run()));
            warn!("VMM spawn_blocking: run() returned (is_ok={})", result.is_ok());

            match result {
                Ok(inner) => inner,
                Err(panic_info) => {
                    let msg = format!("VMM panicked: {:?}", panic_info);
                    error!("{msg}");
                    Err(anyhow::anyhow!(msg))
                },
            }
        })
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

        let loop_start = std::time::Instant::now();

        // Pvclock: VMM-driven system_time updates.
        // Version starts at 2 (set by setup_pvclock). Each update does +1
        // (odd = writing) then +1 (even = stable).
        let mut pvclock_version: u32 = 2;

        // Set Windows multimedia timer resolution to 1ms for accurate
        // thread::sleep in the idle handler.
        unsafe { crate::vmm::whp::timer::timeBeginPeriod(1) };

        // Start a pvclock-refresh thread that periodically cancels the vCPU.
        //
        // Guest kcalls use `int 0x80` (handled internally by the guest IDT),
        // which does NOT cause a VM exit. During kcall-intensive workloads
        // (e.g., many `sched_yield` calls in thread tests), the vCPU can run
        // for seconds without exiting, starving the VMM loop of iterations.
        // This means pvclock never advances and scheduler admission-times
        // become stale.
        //
        // The refresh thread calls `WHvCancelRunVirtualProcessor` every 10ms,
        // forcing the vCPU to exit with `Interrupted` reason. The VMM loop
        // then updates pvclock and re-enters. This is safe because:
        //  - No LAPIC interrupt injection is involved (avoids WHvRequestInterrupt bugs).
        //  - The VMM loop already handles the `Interrupted` exit by continuing.
        let cancel_stop = Arc::new(AtomicBool::new(false));
        let cancel_stop_clone = cancel_stop.clone();
        let cancel_partition = self.partition_handle;
        let cancel_thread = std::thread::spawn(move || {
            let period = std::time::Duration::from_millis(10);
            while !cancel_stop_clone.load(Ordering::Relaxed) {
                std::thread::sleep(period);
                if cancel_stop_clone.load(Ordering::Relaxed) {
                    break;
                }
                // SAFETY: `cancel_partition` is a valid WHP partition handle that
                // outlives this thread (the Vmm struct owns it). vCPU index 0 is
                // always valid for single-vCPU partitions. This call is safe to
                // invoke from any thread without holding the vCPU Mutex.
                let result = unsafe {
                    windows::Win32::System::Hypervisor::WHvCancelRunVirtualProcessor(
                        cancel_partition,
                        0,
                        0,
                    )
                };
                if let Err(e) = result {
                    error!("WHvCancelRunVirtualProcessor failed: {e:?}");
                    break;
                }
            }
        });

        let result = loop {
            // Check shutdown flag.
            if SHUTDOWN.with(|shutdown| shutdown.load(Ordering::SeqCst)) {
                let exit_status: u16 = 0;
                warn!("VMM exit: SHUTDOWN flag (elapsed={:?})", loop_start.elapsed());
                Handle::current().block_on(self.handle_shutdown(exit_status));
                break Ok(exit_status);
            }

            // Update pvclock before every vCPU entry so the guest always
            // reads a fresh wall-clock value from the pvclock page.
            Self::update_pvclock(
                &mut self.vmem.blocking_lock(),
                &mut pvclock_version,
                loop_start.elapsed().as_nanos() as u64,
            );

            // Consume pending IKC notifications silently. On WHP with LAPIC
            // emulation (XApic mode), both inject_pending_interruption and
            // WHvRequestInterrupt trigger ACCESS_VIOLATION (0xC0000005) or
            // deadlock. The IKC interrupt is only a wake-up hint — the guest
            // kernel already polls IKC messages on every kcall iteration via
            // poll_ikc_messages(), so no interrupt injection is needed.
            let _ = self.ikc_notifier.take_pending();

            let exit_context = {
                let mut locked_vcpu: MutexGuard<'_, VirtualProcessor> = self.vcpu.blocking_lock();
                // Exit if the vCPU is no longer online.
                if !locked_vcpu.is_online() {
                    warn!(
                        "VMM exit: vCPU offline (exit_status={}, elapsed={:?})",
                        locked_vcpu.exit_status(),
                        loop_start.elapsed()
                    );
                    break Ok(locked_vcpu.exit_status());
                }
                locked_vcpu.run()
            };

            // Parse exit reason.
            match exit_context.reason_ref() {
                // The guest requested to access an I/O port.
                VirtualProcessorExitReasonRef::PmioAccess(access) => {
                    // Fast-path: handle legacy hardware ports inline.
                    if let PmioAccess::PmioOut(port, data, _width) = access {
                        // PIC, PIT, speaker, IMCR, CMOS, serial: no-op.
                        match *port {
                            0x20
                            | 0x21
                            | 0xA0
                            | 0xA1
                            | 0x22
                            | 0x23
                            | 0x40..=0x43
                            | 0x61
                            | 0x70
                            | 0x71
                            | 0x3F8..=0x3FF => {
                                continue;
                            },
                            _ => {},
                        }

                        // Paravirtualized idle: the guest kernel writes the
                        // IDLE command to the VMM port instead of HLT. Sleep
                        // for one timer period so pvclock advances, then
                        // re-enter. No interrupt injection is needed — the
                        // kernel's scheduler calls check_alarm(clock::now())
                        // on each iteration, and clock::now() reads pvclock
                        // which the VMM loop updates before every vCPU entry.
                        //
                        // WHvRequestInterrupt and PendingInterruption both
                        // trigger WHP bugs (deadlock and ACCESS_VIOLATION
                        // respectively) when LAPIC emulation is enabled, so
                        // this pvclock-only approach avoids all WHP
                        // concurrency issues.
                        if *port == ::config::microvm::DEFAULT_VMM_PORT {
                            let cmd = (*data >> 16) as u16;
                            if cmd == ::config::microvm::DEFAULT_VMM_IDLE_CMD {
                                std::thread::sleep(std::time::Duration::from_micros(
                                    ::config::microvm::TIMER_PERIOD_US,
                                ));
                                continue;
                            }
                        }
                    }
                    if let PmioAccess::PmioIn(port, _data) = access {
                        match *port {
                            0x20
                            | 0x21
                            | 0x22
                            | 0x23
                            | 0xA0
                            | 0xA1
                            | 0x40..=0x43
                            | 0x61
                            | 0x70
                            | 0x71
                            | 0x3F8..=0x3FF
                            | 0xCF8
                            | 0xCFC..=0xCFF => {
                                continue;
                            },
                            _ => {},
                        }
                    }

                    // Slow path: application-level I/O (stdout, stdin, VMM port).

                    let exit_status: Option<u16> = match self
                        .inner
                        .blocking_lock()
                        .emulator
                        .handle_pmio_access(access)
                    {
                        Ok(status) => status,
                        Err(e) => {
                            break Err(e);
                        },
                    };
                    if let Some(exit_status) = exit_status {
                        if exit_status != ::config::microvm::DEFAULT_VMM_PAUSE_CMD {
                            warn!(
                                "VMM exit: PMIO shutdown (exit_status={exit_status}, elapsed={:?})",
                                loop_start.elapsed()
                            );
                            Handle::current().block_on(self.handle_shutdown(exit_status));
                            break Ok(exit_status);
                        } else {
                            if let Err(e) = Handle::current().block_on(self.handle_pause()) {
                                break Err(e);
                            }
                            trace!("VMM resumed");
                        }
                    }
                },

                // HLT: with LAPIC emulation enabled, HLT is handled
                // internally by the LAPIC and shouldn't cause VM exits.
                // If it does, just re-enter.
                VirtualProcessorExitReasonRef::Halt => {
                    continue;
                },

                // The vCPU was canceled (WHvCancelRunVP) or woken by
                // WHvRequestInterrupt from the timer thread. Re-enter
                // after pvclock update (handled at top of loop).
                VirtualProcessorExitReasonRef::Interrupted => {
                    continue;
                },

                // InterruptWindow: IF just became 1. On WHP with LAPIC
                // emulation, inject_pending_interruption is unsafe (triggers
                // ACCESS_VIOLATION), so we just clear the notification and
                // consume any pending IKC flag. The guest kernel polls IKC
                // messages on every kcall iteration, so no injection needed.
                VirtualProcessorExitReasonRef::InterruptWindow => {
                    self.vcpu
                        .blocking_lock()
                        .set_deliverability_notifications(false);
                    let _ = self.ikc_notifier.take_pending();
                    continue;
                },

                // Virtual machine exited due to an unknown reason.
                VirtualProcessorExitReasonRef::Unknown => {
                    warn!("VMM exit: Unknown exit reason (elapsed={:?})", loop_start.elapsed());
                    break Ok(ErrorCode::IllegalByteSequence.into());
                },

                // Guest accessed an unmapped guest physical address.
                // Lazily map a zeroed page so the instruction succeeds on retry.
                VirtualProcessorExitReasonRef::MmioAccess(gpa) => {
                    if let Err(e) = self
                        .vmem
                        .blocking_lock()
                        .map_mmio_page(&self.inner.blocking_lock()._partition, gpa)
                    {
                        break Err(e);
                    }
                    continue;
                },
            }
        };

        // Stop the pvclock-refresh thread.
        cancel_stop.store(true, Ordering::Relaxed);
        let _ = cancel_thread.join();

        // Restore Windows multimedia timer resolution.
        unsafe { crate::vmm::whp::timer::timeEndPeriod(1) };

        warn!("VMM run loop finished (result={result:?}, elapsed={:?})", loop_start.elapsed());
        result
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the virtual memory of the target virtual machine.
    ///
    pub fn vmem(&self) -> Arc<Mutex<VirtualMemory>> {
        self.vmem.clone()
    }

    ///
    /// # Description
    ///
    /// Returns a reference to the guest of the virtual machine.
    ///
    pub fn guest(&self) -> Arc<Mutex<Guest>> {
        self.guest.clone()
    }

    /// Returns a clone of the IKC notifier for use by the memory thread.
    pub fn ikc_notifier(&self) -> IkcNotifier {
        self.ikc_notifier.clone()
    }

    ///
    /// # Description
    ///
    /// Stub for snapshot creation (not supported on WHP backend).
    ///
    pub async fn create_snapshot(&self, _filepath: String) -> Result<()> {
        warn!("create_snapshot(): snapshots are not supported on WHP backend");
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Stub for snapshot loading (not supported on WHP backend).
    ///
    pub async fn load_snapshot(&self, _filepath: String) -> Result<()> {
        warn!("load_snapshot(): snapshots are not supported on WHP backend");
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Sends the vCPU thread's tid to the main thread.
    ///
    async fn send_tid(&self, tid: u64) -> Result<()> {
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
    /// Acknowledges a pause request and waits for the next command.
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
            Some(VcpuControlCommand::CreateSnapshot(_filepath)) => {
                // Snapshots are not supported on Windows WHP backend.
                warn!("handle_pause(): snapshot creation not supported on WHP backend");
                Ok(())
            },
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
    /// the orchestrator thread.
    ///
    async fn handle_shutdown(&mut self, exit_status: u16) {
        // Mark the IKC notifier as shut down so that any pending or future notify() calls from
        // the memory thread will skip the WHvCancelRunVirtualProcessor API call. This prevents
        // STATUS_ACCESS_VIOLATION crashes when the vCPU is no longer running.
        self.ikc_notifier.mark_shutdown();

        // Stop the timer before shutting down.
        self.timer.lock().unwrap().stop();

        // Power-off vCPU.
        self.vcpu.lock().await.poweroff(exit_status);

        // Send message to orchestrator thread.
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
            },
        }
    }

    /// Populates the pvclock shared memory page so the kernel reads accurate
    /// wall-clock time without relying on TSC.
    ///
    /// The page at `DEFAULT_PVCLOCK_PAGE` (GPA 0x1000) uses the KVM pvclock
    /// format (`KvmPvclockVcpuTimeInfo`). On KVM this is filled by the
    /// hypervisor itself; on WHP we write it from the VMM.
    ///
    /// **Strategy**: Set `tsc_to_system_mul = 0` so the TSC delta contributes
    /// nothing. The VMM periodically updates `system_time` directly from the
    /// host clock (see [`update_pvclock`]). This avoids depending on the
    /// guest's `rdtsc` matching the host's TSC — which is unreliable on WHP.
    fn setup_pvclock(vmem: &mut VirtualMemory) -> Result<()> {
        use std::time::{
            SystemTime,
            UNIX_EPOCH,
        };

        // Build the KvmPvclockVcpuTimeInfo structure (32 bytes).
        //   [0..4]   version: u32           (2 = valid, even)
        //   [4..8]   _pad0: u32             (0)
        //   [8..16]  tsc_timestamp: u64     (unused, 0)
        //   [16..24] system_time: u64       (ns, updated by VMM)
        //   [24..28] tsc_to_system_mul: u32 (0 = TSC not used)
        //   [28]     tsc_shift: i8          (0)
        //   [29]     flags: u8              (0x01 = TSC_STABLE)
        //   [30..32] _pad: [u8; 2]
        let mut data = [0u8; 32];
        // Version 2 = pvclock enabled (VMM-driven system_time updates).
        let pvclock_version: u32 = 2;
        data[0..4].copy_from_slice(&pvclock_version.to_le_bytes());
        // tsc_timestamp = 0, system_time = 0, mul = 0, shift = 0
        data[29] = 0x01; // TSC_STABLE

        let pvclock_gpa = ::config::microvm::DEFAULT_PVCLOCK_PAGE as u64;
        vmem.write_bytes(pvclock_gpa, &data)?;

        // Write boot time (UTC nanoseconds since Unix epoch) at offset 0x20.
        let boot_time_ns: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let boot_offset = pvclock_gpa + ::config::microvm::PVCLOCK_BOOT_TIME_NS_OFFSET as u64;
        vmem.write_bytes(boot_offset, &boot_time_ns.to_le_bytes())?;

        trace!("pvclock: VMM-driven mode (mul=0), boot_ns={boot_time_ns}");

        Ok(())
    }

    /// Updates the pvclock page's `system_time` field via the seqlock protocol.
    ///
    /// The kernel's seqlock reader retries when it sees an odd version, so
    /// the update sequence is: version→odd, write system_time, version→even.
    fn update_pvclock(vmem: &mut VirtualMemory, pvclock_version: &mut u32, system_time_ns: u64) {
        let gpa = ::config::microvm::DEFAULT_PVCLOCK_PAGE as u64;
        // Odd version signals "update in progress".
        *pvclock_version += 1;
        let _ = vmem.write_bytes(gpa, &pvclock_version.to_le_bytes());
        // Update system_time (offset 16).
        let _ = vmem.write_bytes(gpa + 16, &system_time_ns.to_le_bytes());
        // Even version signals "stable snapshot".
        *pvclock_version += 1;
        let _ = vmem.write_bytes(gpa, &pvclock_version.to_le_bytes());
    }
}
