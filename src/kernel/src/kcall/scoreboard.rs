// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    kcall::{
        KcallArgs,
        KcallResult,
    },
    pm::{
        sync::{
            condvar::Condvar,
            semaphore::Semaphore,
        },
        SleepError,
    },
};
use ::config::kernel::SCOREBOARD_SLOTS;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Global Variables
//==================================================================================================

///
/// # Description
///
/// Global scoreboard used to coordinate kernel call dispatching and completion signaling between
/// user and kernel threads.
///
static mut SCOREBOARD: Option<ScoreBoard> = None;

//==================================================================================================
// Scoreboard
//==================================================================================================

///
/// # Description
///
/// Coordinates kernel call dispatching and completion signaling between user and kernel threads.
///
pub struct ScoreBoard {
    /// Synchronizes number slots available for kernel call dispatching.
    available_slots: Semaphore,
    /// Synchronizes number of kernel calls ready to be handled.
    pending_kcalls: Semaphore,
    /// Slots for kernel call dispatching.
    slots: [ScoreBoardSlot; SCOREBOARD_SLOTS],
    /// Next slot index to inspect when searching for pending kernel calls.
    next_handle_index: usize,
}

impl ScoreBoard {
    ///
    /// # Description
    ///
    /// Initializes the global scoreboard.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates the global scoreboard without explicit
    /// synchronization.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The caller is running with interrupts disabled when invoking this function.
    ///
    pub unsafe fn init() {
        SCOREBOARD = Some(ScoreBoard {
            available_slots: Semaphore::new(SCOREBOARD_SLOTS),
            pending_kcalls: Semaphore::new(0),
            slots: ::core::array::from_fn(|_| ScoreBoardSlot::new()),
            next_handle_index: 0,
        });
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the global scoreboard instance.
    ///
    /// # Returns
    ///
    /// On successful completion, this function returns a mutable reference to the global scoreboard.
    /// On failure, it returns an object that describes the error encountered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it provides direct mutable access to the global scoreboard
    /// without explicit synchronization.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The caller is running with interrupts disabled when invoking this function.
    ///
    unsafe fn get_mut() -> Result<&'static mut ScoreBoard, Error> {
        if let Some(scoreboard) = SCOREBOARD.as_mut() {
            Ok(scoreboard)
        } else {
            let reason: &str = "uninitialized scoreboard";
            error!("{reason}");
            Err(Error::new(ErrorCode::TryAgain, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Dispatches a kernel call to be handled by the kernel thread.
    ///
    /// # Parameters
    ///
    /// - `number`: Kernel call number to execute.
    /// - `pid`: Identifier of the calling process.
    /// - `tid`: Identifier of the calling thread.
    /// - `arg0`: First kernel call argument.
    /// - `arg1`: Second kernel call argument.
    /// - `arg2`: Third kernel call argument.
    /// - `arg3`: Fourth kernel call argument.
    ///
    /// # Return
    ///
    /// On successful completion, this function returns the result of the kernel call. On failure,
    /// it returns an object that describes the error encountered.
    ///
    /// # Safety
    ///
    /// This function is unsafe because:
    /// - It mutates the global scoreboard without explicit synchronization.
    /// - It suspends the execution of the calling thread until the kernel call is handled.
    /// - It must be called only by a user thread.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The caller is running with interrupts disabled when invoking this function.
    /// - The caller does not hold kernel resources while waiting on the kernel call to complete.
    /// - The caller must be a user thread.
    ///
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn dispatch(
        number: u32,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
    ) -> Result<KcallResult, SleepError> {
        let scoreboard: &'static mut ScoreBoard =
            unsafe { Self::get_mut() }.map_err(SleepError::Generic)?;

        // Attempt to reclaim any aborted slots. If we fail, we don't proceed with the dispatching,
        // as we could block indefinitely.
        scoreboard.reclaim_aborted_slots()?;

        // Wait for a scoreboard slot to become available.
        if let Err(error) = unsafe { scoreboard.available_slots.down() } {
            let reason: &str = "no available scoreboard slots";
            error!("{reason}");
            return Err(error);
        }

        // Find slot that became available.
        let slot_index: usize = match scoreboard
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_free())
        {
            // Found a free slot.
            Some((index, slot)) => {
                slot.fill(number, pid, tid, arg0, arg1, arg2, arg3);
                index
            },
            // No free slot found.
            None => {
                // This is unlikely to happen, as we just waited for one above. Still, instead of
                // panicking, we fail gracefully. Also, we intentionally do not signal the
                // `available_slots` semaphore, as no slot was actually acquired.
                let reason: &str = "no free scoreboard slots";
                error!("{reason}");
                return Err(SleepError::Generic(Error::new(ErrorCode::TryAgain, reason)));
            },
        };

        // Signal that a new kernel call is pending.
        let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[slot_index];

        if let Err(error) = unsafe { scoreboard.pending_kcalls.up() } {
            let reason: &str = "failed to signal pending kernel calls";
            error!("{reason} (error={error:?})");

            // Attempt to rollback the slot acquisition. We first attempt to signal that a slot
            // became available and only if that succeeds we mark the slot as free again. If the
            // signaling fails, the slot is marked as aborted, preventing other dispatchers from
            // reusing it prematurely.
            if let Err(rollback_error) = unsafe { scoreboard.available_slots.up() } {
                let rollback_reason: &str = "failed to rollback available scoreboard slots";
                warn!("{rollback_reason}: {rollback_error:?}");
                slot.mark_aborted(AbortedDisposition::BeforeSignal);
            } else {
                slot.mark_free();
            }

            return Err(SleepError::Generic(error));
        }

        // Mark the slot as signaled after successfully incrementing the pending_kcalls semaphore.
        slot.mark_signaled();

        // Wait for the kernel call to be handled.
        let wait_result: Result<(), SleepError> = {
            let handled: &Condvar = &scoreboard.slots[slot_index].handled;
            unsafe { handled.wait(None) }
        };

        // Get the slot.
        let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[slot_index];

        if let Err(error) = wait_result {
            slot.mark_aborted(AbortedDisposition::AfterSignal);
            let reason: &str = "failed to wait for kernel call handling";
            warn!("{reason} (error={error:?})");
            return Err(error);
        }

        // Retrieve the result.
        let result: Option<KcallResult> = slot.result.take();

        // Attempt to signal slot availability. If we fail, we mark the slot as aborted to prevent
        // other dispatchers from reusing it prematurely. We do not fail the dispatching at this
        // point, as the kernel call was already handled successfully. Note that unlike the earlier
        // failure path at pending_kcalls.up(), we do not attempt to roll back the result retrieval
        // because the kernel call has already completed and the result must be returned to the
        // caller.
        if let Err(error) = unsafe { scoreboard.available_slots.up() } {
            slot.mark_aborted(AbortedDisposition::BeforeSignal);
            let reason: &str = "failed to signal available scoreboard slots";
            warn!("{reason} (error={error:?})");
        } else {
            slot.mark_free();
        }

        match result {
            Some(ret) => Ok(ret),
            None => {
                let reason: &str = "missing kernel call result";
                error!("{reason}");
                Err(SleepError::Generic(Error::new(ErrorCode::TryAgain, reason)))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Acquires the next pending slot and returns its arguments for processing.
    ///
    /// # Return
    ///
    /// On successful completion, this function returns the slot index paired with its corresponding
    /// kernel arguments. On failure, it returns an object that describes the error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because:
    /// - It mutates the global scoreboard without explicit synchronization.
    /// - It must be called only by the kernel thread.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The caller is running with interrupts disabled when invoking this function.
    /// - The caller must be the kernel thread.
    ///
    pub unsafe fn handle() -> Result<(ScoreBoardSlotIndex, &'static KcallArgs), Error> {
        let scoreboard: &'static mut ScoreBoard = unsafe { Self::get_mut() }?;

        // Wait for a pending kernel call.
        if let Err(error) = scoreboard.pending_kcalls.try_down() {
            // Only log actual errors. ErrorCode::TryAgain is the expected non-error case when no
            // kernel calls are pending, so we skip logging for it.
            if error.code != ErrorCode::TryAgain {
                error!("failed to wait for pending kernel calls (error={error:?})");
            }
            return Err(error);
        }

        // Find a pending slot, resuming from the previously handled index to avoid starvation.
        let total_slots: usize = scoreboard.slots.len();
        let start_index: usize = scoreboard.next_handle_index;
        let mut found_slot: Option<(usize, *const KcallArgs)> = None;

        for offset in 0..total_slots {
            let current_index: usize = (start_index + offset) % total_slots;
            let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[current_index];
            if slot.is_pending() {
                found_slot = Some((current_index, &slot.args as *const KcallArgs));
                scoreboard.next_handle_index = (current_index + 1) % total_slots;
                break;
            }
        }

        let (slot_index, args_ptr): (usize, *const KcallArgs) = match found_slot {
            Some(tuple) => tuple,
            None => {
                let reason: &str = "no dispatched slot available";
                error!("{reason}");

                let original_error: Error = Error::new(ErrorCode::TryAgain, reason);

                if let Err(error) = unsafe { scoreboard.pending_kcalls.up() } {
                    let rollback_reason: &str = "failed to rollback pending kernel calls";
                    warn!("{rollback_reason}: {error:?}");
                }

                return Err(original_error);
            },
        };

        let args: &'static KcallArgs = unsafe { &*args_ptr };

        Ok((ScoreBoardSlotIndex(slot_index), args))
    }

    ///
    /// # Description
    ///
    /// Stores the handler result back into the referenced slot and wakes the waiting dispatcher.
    ///
    /// # Parameters
    ///
    /// - `index`: Slot index that was previously obtained via `handle()`.
    /// - `ret`: Kernel call result to deliver to the dispatcher.
    ///
    /// # Safety
    ///
    /// This function is unsafe because:
    /// - It mutates the global scoreboard without explicit synchronization.
    /// - It must be called only by the kernel thread.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The caller is running with interrupts disabled when invoking this function.
    /// - The caller must be the kernel thread.
    ///
    pub unsafe fn handled(index: ScoreBoardSlotIndex, ret: KcallResult) -> Result<(), Error> {
        let scoreboard: &'static mut ScoreBoard = unsafe { Self::get_mut() }?;

        // Get the slot.
        let slot: &mut ScoreBoardSlot =
            scoreboard.slots.get_mut(index.as_usize()).ok_or_else(|| {
                let reason: &str = "invalid scoreboard slot index";
                error!("{reason}");
                Error::new(ErrorCode::InvalidArgument, reason)
            })?;

        match slot.state {
            ScoreBoardSlotState::Signaled => {
                slot.result = Some(ret);

                // Notify the waiting dispatcher.
                if let Err(error) = unsafe { slot.handled.notify_first() } {
                    let reason: &str = "failed to notify waiting dispatcher";
                    error!("{reason}");
                    return Err(error);
                }

                Ok(())
            },
            ScoreBoardSlotState::Aborted(AbortedDisposition::AfterSignal) => {
                slot.result = None;

                // Release the slot back to the available pool. This is safe because when the
                // dispatcher aborted with AfterSignal, it did not release the slot to ensure
                // the handler could process it exactly once. We perform the release here to
                // maintain the invariant that each slot acquisition has exactly one corresponding
                // release.
                if let Err(error) = unsafe { scoreboard.available_slots.up() } {
                    slot.mark_aborted(AbortedDisposition::BeforeSignal);
                    let reason: &str = "failed to recycle aborted scoreboard slot";
                    error!("{reason}");
                    return Err(error);
                }

                slot.mark_free();

                Ok(())
            },
            ScoreBoardSlotState::Aborted(AbortedDisposition::BeforeSignal) => {
                slot.result = None;
                slot.mark_free();
                warn!("kernel handled slot aborted before signaling");
                Ok(())
            },
            _ => {
                let reason: &str = "slot is not in a signalable state";
                error!("{reason}");
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Reclaims slots that were marked as aborted by dispatchers and releases them back to the
    /// available pool.
    ///
    /// # Return
    ///
    /// On successful completion, this function returns empty. Otherwise, it returns a sleep error
    /// describing why the reclamation failed.
    ///
    fn reclaim_aborted_slots(&mut self) -> Result<(), SleepError> {
        for slot in self.slots.iter_mut() {
            if slot.aborted_before_signal() {
                if let Err(error) = unsafe { self.available_slots.up() } {
                    let reason: &str = "failed to recycle aborted scoreboard slot";
                    error!("{reason}");
                    return Err(SleepError::Generic(error));
                }
                slot.mark_free();
            }
        }

        Ok(())
    }
}

//==================================================================================================
// Scoreboard State
//==================================================================================================

///
/// # Description
///
/// Represents lifecycle states for a scoreboard slot.
///
#[derive(Clone, Copy, PartialEq, Eq)]
enum ScoreBoardSlotState {
    /// Slot is available for a new kernel call.
    Free,
    /// Slot has been acquired but not yet signaled to the kernel.
    InUse,
    /// Slot has been signaled and is ready for the kernel to process.
    Signaled,
    /// Slot was abandoned by the dispatcher and requires reclamation.
    Aborted(AbortedDisposition),
}

///
/// # Description
///
/// Captures how an aborted slot should be reclaimed.
///
#[derive(Clone, Copy, PartialEq, Eq)]
enum AbortedDisposition {
    /// Slot was aborted before signaling the pending kernel calls semaphore.
    BeforeSignal,
    /// Slot was aborted after signaling the pending kernel calls semaphore.
    AfterSignal,
}

//==================================================================================================
// Scoreboard Slot Index
//==================================================================================================

///
/// # Description
///
/// Identifies a slot within the scoreboard storage.
///
/// This is a wrapper around a `usize` that provides type safety and prevents accidental misuse of
/// raw indices. It is returned by `ScoreBoard::handle()` and must be passed back to
/// `ScoreBoard::handled()` to complete the kernel call handling cycle.
///
pub struct ScoreBoardSlotIndex(usize);

impl ScoreBoardSlotIndex {
    ///
    /// # Description
    ///
    /// Converts the slot index into a `usize` for array access.
    ///
    fn as_usize(&self) -> usize {
        self.0
    }
}

//==================================================================================================
// Scoreboard Slot
//==================================================================================================

///
/// # Description
///
/// Tracks the arguments, result, and synchronization primitive for a single slot.
///
struct ScoreBoardSlot {
    /// Current lifecycle state of the slot.
    state: ScoreBoardSlotState,
    /// Kernel call arguments provided by the dispatcher.
    args: KcallArgs,
    /// Kernel call result set by the handler, or `None` if not yet handled.
    result: Option<KcallResult>,
    /// Condition variable used to wake the dispatcher when the kernel call is handled.
    handled: Condvar,
}

impl ScoreBoardSlot {
    ///
    /// # Description
    ///
    /// Creates a scoreboard slot initialized with sentinel arguments.
    ///
    /// # Return
    ///
    /// This function returns a new instance of a scoreboard slot.
    ///
    fn new() -> Self {
        ScoreBoardSlot {
            state: ScoreBoardSlotState::Free,
            args: KcallArgs {
                pid: ProcessIdentifier::from(i32::MAX),
                tid: ThreadIdentifier::from(i32::MAX),
                arg0: 0,
                arg1: 0,
                arg2: 0,
                arg3: 0,
                number: 0,
            },
            result: None,
            handled: Condvar::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether the slot is currently free.
    ///
    /// # Return
    ///
    /// True if the slot is free, false otherwise.
    ///
    fn is_free(&self) -> bool {
        matches!(self.state, ScoreBoardSlotState::Free)
    }

    ///
    /// # Description
    ///
    /// Checks whether the slot is pending processing.
    ///
    /// # Return
    ///
    /// True if the slot is pending processing, false otherwise.
    ///
    fn is_pending(&self) -> bool {
        matches!(
            self.state,
            ScoreBoardSlotState::Signaled
                | ScoreBoardSlotState::Aborted(AbortedDisposition::AfterSignal)
        ) && self.result.is_none()
    }

    ///
    /// # Description
    ///
    /// Fills the slot with kernel call arguments and marks it as in use.
    ///
    /// # Parameters
    ///
    /// - `number`: Kernel call number to execute.
    /// - `pid`: Identifier of the calling process.
    /// - `tid`: Identifier of the calling thread.
    /// - `arg0`: First kernel call argument.
    /// - `arg1`: Second kernel call argument.
    /// - `arg2`: Third kernel call argument.
    /// - `arg3`: Fourth kernel call argument.
    ///
    #[allow(clippy::too_many_arguments)]
    fn fill(
        &mut self,
        number: u32,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
    ) {
        self.state = ScoreBoardSlotState::InUse;
        self.args = KcallArgs {
            pid,
            tid,
            arg0,
            arg1,
            arg2,
            arg3,
            number,
        };
        self.result = None;
    }

    ///
    /// # Description
    ///
    /// Marks the slot as free and clears any pending result.
    ///
    fn mark_free(&mut self) {
        self.state = ScoreBoardSlotState::Free;
        self.result = None;
    }

    ///
    /// # Description
    ///
    /// Marks the slot as aborted so the kernel handler can recycle it safely.
    ///
    fn mark_aborted(&mut self, disposition: AbortedDisposition) {
        self.state = ScoreBoardSlotState::Aborted(disposition);
        self.result = None;
    }

    ///
    /// # Description
    ///
    /// Marks the slot as visible to the kernel thread after the pending semaphore is signaled.
    ///
    fn mark_signaled(&mut self) {
        self.state = ScoreBoardSlotState::Signaled;
    }

    ///
    /// # Description
    ///
    /// Checks whether the slot was aborted before the kernel handler observed it.
    ///
    fn aborted_before_signal(&self) -> bool {
        matches!(self.state, ScoreBoardSlotState::Aborted(AbortedDisposition::BeforeSignal))
    }
}
