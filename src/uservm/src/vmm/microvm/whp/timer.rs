// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// The WHP timer uses size_of<T>() as u32 for the Windows API.
#![allow(clippy::cast_possible_truncation)]

//==================================================================================================
// Imports
//==================================================================================================

use super::RunState;
use ::log::trace;
use ::std::{
    sync::{
        Arc,
        Condvar,
        Mutex,
        MutexGuard,
        WaitTimeoutResult,
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
/// `WHvCancelRunVirtualProcessor` when a vCPU run is pending to cause a `Canceled` exit. The VMM
/// loop uses these exits to update `system_time` on the pvclock page.
///
/// The inter-tick wait is **interruptible**: the thread waits on a
/// [`Condvar`] with the tick period as a timeout, rather than calling
/// `thread::sleep`. This lets [`Timer::stop`] wake the thread (and thus
/// return from `join`) immediately, instead of blocking for the
/// remainder of the current tick. A non-interruptible `thread::sleep`
/// made VM teardown stall for up to a full tick period (~10 ms),
/// inflating measured boot latency.
///
pub struct Timer {
    /// WHP partition handle (for `WHvCancelRunVirtualProcessor`).
    partition: WHV_PARTITION_HANDLE,
    /// State that limits cancellation to an active or pending vCPU run.
    run_state: Arc<RunState>,
    /// Shared stop flag (`true` once the timer should exit) and the
    /// [`Condvar`] used to wake the timer thread promptly on stop.
    stop: Arc<(Mutex<bool>, Condvar)>,
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
    pub(super) fn new(partition: WHV_PARTITION_HANDLE, run_state: Arc<RunState>) -> Self {
        Self {
            partition,
            run_state,
            stop: Arc::new((Mutex::new(false), Condvar::new())),
            thread: None,
        }
    }

    /// Starts the timer thread with the given period in microseconds.
    ///
    /// Each tick cancels a pending vCPU run to force a VM exit so the VMM loop can update pvclock.
    /// No guest interrupt is injected.
    pub fn start(&mut self, period_us: u64) {
        if self.thread.is_some() {
            return;
        }

        trace!("Timer::start(): period_us={period_us}");

        // Clear the stop flag before (re)starting the timer thread.
        {
            let (lock, _cvar): &(Mutex<bool>, Condvar) = &self.stop;
            *lock.lock().unwrap() = false;
        }

        let stop: Arc<(Mutex<bool>, Condvar)> = self.stop.clone();
        let partition: WHV_PARTITION_HANDLE = self.partition;
        let run_state: Arc<RunState> = self.run_state.clone();
        let period: Duration = Duration::from_micros(period_us);

        self.thread = Some(thread::spawn(move || {
            // Request 1 ms timer resolution so the periodic wait is reasonably accurate.
            unsafe { timeBeginPeriod(1) };

            let (lock, cvar): &(Mutex<bool>, Condvar) = &stop;
            let mut stopped: MutexGuard<'_, bool> = lock.lock().unwrap();
            while !*stopped {
                // Interruptible wait: returns early (without timing out) when `stop()`
                // notifies, so teardown does not block for the rest of the tick period.
                // `wait_timeout_while` absorbs spurious wakeups and adjusts the remaining
                // timeout internally, keeping the tick cadence stable. It reports
                // `timed_out() == false` only when `stop()` set the flag; a `true` result
                // means the full period elapsed and it is time to fire a pvclock cancel.
                let (guard, result): (MutexGuard<'_, bool>, WaitTimeoutResult) = cvar
                    .wait_timeout_while(stopped, period, |stopped| !*stopped)
                    .unwrap();
                if !result.timed_out() {
                    // Woken by `stop()`.
                    break;
                }
                // Period elapsed: release the lock while cancelling so `stop()` can proceed.
                drop(guard);
                run_state.cancel_if_pending(|| {
                    // SAFETY: `partition` is a valid WHP partition handle that outlives
                    // the timer thread (the Vmm struct owns both).
                    unsafe {
                        let _: Result<(), windows::core::Error> =
                            WHvCancelRunVirtualProcessor(partition, 0, 0);
                    }
                });
                stopped = lock.lock().unwrap();
            }

            unsafe { timeEndPeriod(1) };
            trace!("Timer thread exiting");
        }));
    }

    /// Stops the timer thread, waiting for it to finish.
    ///
    /// Signals the stop flag and notifies the timer thread so it wakes from its
    /// interruptible wait immediately; `join` therefore returns without blocking
    /// for the remainder of the current tick.
    pub fn stop(&mut self) {
        if let Some(thread) = self.thread.take() {
            trace!("Timer::stop()");
            let (lock, cvar) = &*self.stop;
            {
                *lock.lock().unwrap() = true;
            }
            cvar.notify_all();
            let _ = thread.join();
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.stop();
    }
}
