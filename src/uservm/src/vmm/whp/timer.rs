// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::trace;
use ::std::{
    mem,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering,
        },
    },
    thread::{
        self,
        JoinHandle,
    },
    time::Duration,
};
use windows::Win32::System::Hypervisor::{
    WHV_INTERRUPT_CONTROL,
    WHV_PARTITION_HANDLE,
    WHvRequestInterrupt,
};

// Windows Multimedia timer functions for high-resolution sleep.
#[link(name = "winmm")]
unsafe extern "system" {
    pub(super) fn timeBeginPeriod(uPeriod: u32) -> u32;
    pub(super) fn timeEndPeriod(uPeriod: u32) -> u32;
}

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A host-side timer that periodically injects a timer interrupt into
/// the guest vCPU via `WHvRequestInterrupt`. This replaces the earlier
/// two-stage mechanism (cancel vCPU + inject at idle port exit) with
/// direct LAPIC-level interrupt injection from the timer thread.
///
/// The timer thread fires at the guest-requested period and injects
/// IRQ0 (vector 0x20) as a Fixed, Edge-triggered interrupt through
/// `WHvRequestInterrupt`. The WHP LAPIC emulator handles IF checks,
/// IRR/ISR management, and HLT wake-up — so the guest can use a
/// standard `sti; hlt` idle loop.
///
/// A separate clock-refresh thread in the VMM module handles periodic
/// `WHvCancelRunVirtualProcessor` calls for pvclock updates, IKC
/// delivery, and shutdown checks.
///
pub struct Timer {
    /// WHP partition handle (for `WHvRequestInterrupt`).
    partition: WHV_PARTITION_HANDLE,
    /// Flag used to signal the timer thread to stop.
    stop: Arc<AtomicBool>,
    /// Handle to the background timer thread (if running).
    thread: Option<JoinHandle<()>>,
}

// SAFETY: `WHV_PARTITION_HANDLE` is an opaque OS handle that can be used from
// any thread. The `Timer` struct has no thread-affine state.
unsafe impl Send for Timer {}
unsafe impl Sync for Timer {}

//==================================================================================================
// Implementations
//==================================================================================================

impl Timer {
    /// Creates a new timer for the given WHP partition. The timer is initially stopped.
    pub fn new(partition: WHV_PARTITION_HANDLE) -> Self {
        Self {
            partition,
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    /// Starts the timer thread with the given period in microseconds.
    ///
    /// Each tick: `WHvRequestInterrupt` injects vector 0x20 (IRQ0)
    /// as a Fixed, Edge-triggered interrupt via the WHP LAPIC. This
    /// wakes the vCPU from HLT and delivers the interrupt when IF=1.
    pub fn start(&mut self, period_us: u64) {
        if self.thread.is_some() {
            return;
        }

        trace!("Timer::start(): period_us={period_us}");
        self.stop.store(false, Ordering::Relaxed);

        let stop = self.stop.clone();
        let partition = self.partition;
        let period = Duration::from_micros(period_us);

        self.thread = Some(thread::spawn(move || {
            // Set Windows timer resolution to 1ms for accurate sleep.
            unsafe { timeBeginPeriod(1) };

            // Build the interrupt control structure for Fixed, Edge-triggered,
            // Physical destination mode, vector 0x20, destination 0 (BSP).
            // Bitfield layout: bits 0-7 = InterruptType (Fixed=0),
            // bit 8 = DestinationMode (Physical=0), bit 9 = TriggerMode (Edge=0).
            let interrupt: WHV_INTERRUPT_CONTROL = WHV_INTERRUPT_CONTROL {
                _bitfield: 0, // Fixed=0, Physical=0, Edge=0.
                Destination: 0,
                Vector: super::lapic::TIMER_VECTOR,
            };
            let interrupt_size: u32 = mem::size_of::<WHV_INTERRUPT_CONTROL>() as u32;

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(period);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // SAFETY: `partition` is a valid WHP partition handle that outlives
                // the timer thread (the Vmm struct owns both). `interrupt` is a valid
                // WHV_INTERRUPT_CONTROL on the stack with correct size.
                unsafe {
                    let _ = WHvRequestInterrupt(partition, &interrupt, interrupt_size);
                }
            }

            unsafe { timeEndPeriod(1) };
            trace!("Timer thread exiting");
        }));
    }

    /// Stops the timer thread, waiting for it to finish.
    pub fn stop(&mut self) {
        if let Some(thread) = self.thread.take() {
            trace!("Timer::stop()");
            self.stop.store(true, Ordering::Relaxed);
            let _ = thread.join();
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop();
    }
}
