// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The WHP timer uses size_of<T>() as u32 for the Windows API.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

use ::log::trace;
use ::std::{
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
    WHV_PARTITION_HANDLE,
    WHvCancelRunVirtualProcessor,
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
/// A host-side timer that periodically cancels the vCPU to force VMM
/// loop re-entry for pvclock updates. This does **not** inject a guest
/// interrupt — the LAPIC periodic timer (configured by the kernel at
/// 1 kHz) handles all timer interrupts inside the WHP LAPIC emulator.
///
/// The timer thread fires at a low frequency (100 Hz) and calls
/// `WHvCancelRunVirtualProcessor` to cause a `Canceled` exit. The VMM
/// loop uses these exits to update `system_time` on the pvclock page.
///
pub struct Timer {
    /// WHP partition handle (for `WHvCancelRunVirtualProcessor`).
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
    /// Each tick calls `WHvCancelRunVirtualProcessor` to force a VM exit
    /// so the VMM loop can update pvclock. No guest interrupt is injected.
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

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(period);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // SAFETY: `partition` is a valid WHP partition handle that outlives
                // the timer thread (the Vmm struct owns both).
                unsafe {
                    let _ = WHvCancelRunVirtualProcessor(partition, 0, 0);
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
