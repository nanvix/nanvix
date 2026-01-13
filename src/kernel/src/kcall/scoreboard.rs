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
        sync::semaphore::Semaphore,
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
/// # Safety
///
/// This global is accessed through [`ScoreBoard::get_mut()`], which returns `&'static mut`.
/// Multiple `&mut` references to the same allocation technically violate Rust's aliasing rules.
/// This is accepted here because the kernel runs on a single core with interrupts disabled
/// (cooperative scheduling), so no two threads execute concurrently. Under this execution model
/// only one `&mut` is active at any program point, making the access pattern sound in practice.
/// If the kernel ever moves to a preemptive or multi-core model, this must be replaced with an
/// `UnsafeCell`-based container that upholds interior-mutability invariants.
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
    /// Synchronizes number of slots available for kernel call dispatching.
    available_slots: Semaphore,
    /// Synchronizes number of kernel calls ready to be handled.
    pending_kcalls: Semaphore,
    /// Slots for kernel call dispatching.
    slots: [ScoreBoardSlot; SCOREBOARD_SLOTS],
    /// Next slot index to inspect when searching for pending kernel calls.
    next_handle_index: usize,
    /// Next slot index to inspect when searching for free slots during dispatch.
    next_dispatch_index: usize,
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
            next_dispatch_index: 0,
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
    /// # Errors
    ///
    /// This function returns an error if the global scoreboard has not been initialized.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it provides direct mutable access to the global scoreboard
    /// without explicit synchronization.
    ///
    /// Although multiple callers (dispatcher threads and the kernel handler thread) may hold
    /// references obtained through this function concurrently, they always operate on disjoint
    /// slot indices:
    /// - A dispatcher thread only touches its own slot (identified by `find_free_slot`).
    /// - The handler thread only touches slots discovered via `handle()`.
    /// - The semaphore protocol ensures a slot transitions through `Free -> InUse -> Signaled`
    ///   on the dispatcher side before the handler observes it, and back to `Free` only after
    ///   the handler has finished.
    ///
    /// The shared fields `next_dispatch_index`, `next_handle_index`, as well as the
    /// `available_slots` / `pending_kcalls` semaphores, are mutated by multiple callers.
    /// This is sound because the kernel uses cooperative scheduling on a single core with
    /// interrupts disabled, so only one caller executes at any program point and no concurrent
    /// mutation of these fields can occur.
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
            Err(Error::new(ErrorCode::InvalidArgument, reason))
        }
    }

    ///
    /// # Description
    ///
    /// Dispatches a kernel call to be handled by the kernel thread.
    ///
    /// # Parameters
    ///
    /// - `args`: Kernel call arguments describing the request.
    ///
    /// # Returns
    ///
    /// On successful completion, this function returns the result of the kernel call. On failure,
    /// it returns an object that describes the error encountered.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The global scoreboard has not been initialized.
    /// - No scoreboard slot is available for dispatching.
    /// - Signaling the pending kernel call fails.
    /// - Waiting for the kernel call result fails (e.g., the dispatcher is aborted).
    /// - The kernel call result is missing after the handler completes.
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
    pub unsafe fn dispatch(args: KcallArgs) -> Result<KcallResult, SleepError> {
        let scoreboard: &'static mut ScoreBoard =
            unsafe { Self::get_mut() }.map_err(SleepError::Generic)?;

        // Wait for a scoreboard slot to become available.
        if let Err(error) = unsafe { scoreboard.available_slots.down() } {
            let reason: &str = "failed to acquire scoreboard slot";
            error!("{reason} (error={error:?})");
            return Err(error);
        }

        // Find slot that became available.
        let slot_index: usize = match scoreboard.find_free_slot(args) {
            // Found a free slot.
            Some(index) => index,
            // No free slot found.
            None => {
                // This is unlikely to happen, as we just waited for one above. Still, instead
                // of panicking, we fail gracefully. Semaphore::up() always increments the
                // counter before calling notify_first(), so even on failure the available slot
                // count is restored.
                let reason: &str = "no free scoreboard slots";
                error!("{reason}");

                if let Err(rollback_error) = unsafe { scoreboard.available_slots.up() } {
                    let rollback_reason: &str =
                        "failed to notify on available scoreboard slots rollback";
                    warn!("{rollback_reason}: {rollback_error:?}");
                }
                return Err(SleepError::Generic(Error::new(ErrorCode::TryAgain, reason)));
            },
        };

        // Mark the slot as signaled before incrementing the pending_kcalls semaphore.
        // This ordering is critical: the handler checks `is_pending()` which requires
        // `Signaled` state. If we signaled the semaphore first, the handler could wake
        // and find the slot still in `InUse` state, missing the pending kernel call.
        let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[slot_index];
        slot.mark_signaled();

        // Signal that a new kernel call is pending.
        if let Err(error) = unsafe { scoreboard.pending_kcalls.up() } {
            let reason: &str = "failed to signal pending kernel calls";
            error!("{reason} (error={error:?})");

            // At this point, Semaphore::up() has already incremented the internal
            // pending_kcalls counter before failing to notify a waiter. To preserve the
            // rendezvous invariant between pending_kcalls and slot states, we must not
            // make this slot appear free again while leaving the semaphore count
            // incremented. Instead, we mark the slot as aborted so that the handler can
            // observe and retire it consistently.
            slot.mark_aborted();

            return Err(SleepError::Generic(error));
        }

        // Wait for the kernel call to be handled.
        let wait_result: Result<(), SleepError> = {
            let handled: &Semaphore = &scoreboard.slots[slot_index].handled;
            unsafe { handled.down() }
        };

        // Get the slot.
        let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[slot_index];

        if let Err(error) = wait_result {
            slot.mark_aborted();
            let reason: &str = "failed to wait for kernel call handling";
            error!("{reason} (error={error:?})");
            return Err(error);
        }

        // Retrieve the result.
        let result: Option<KcallResult> = slot.result.take();

        // Attempt to signal slot availability. Semaphore::up() always increments the internal
        // counter before calling notify_first(), so even on failure the available slot count is
        // already restored. We mark the slot as free regardless because the kernel call was
        // already handled successfully and the counter is consistent.
        if let Err(error) = unsafe { scoreboard.available_slots.up() } {
            let reason: &str = "failed to notify on available scoreboard slots";
            warn!("{reason} (error={error:?})");
        }
        slot.mark_free();

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
    /// # Returns
    ///
    /// On successful completion, this function returns the slot index paired with its corresponding
    /// kernel arguments. On failure, it returns an object that describes the error.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The global scoreboard has not been initialized.
    /// - No kernel calls are pending ([`ErrorCode::TryAgain`]).
    /// - A pending kernel call count exists but no matching slot is found.
    /// - A pending slot was found but had been aborted by the dispatcher; the slot is recycled
    ///   and [`ErrorCode::TryAgain`] is returned.
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
        let mut found_index: Option<usize> = None;

        for offset in 0..total_slots {
            let current_index: usize = (start_index + offset) % total_slots;
            if scoreboard.slots[current_index].is_pending() {
                found_index = Some(current_index);
                scoreboard.next_handle_index = (current_index + 1) % total_slots;
                break;
            }
        }

        let slot_index: usize = match found_index {
            Some(index) => index,
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

        // If the slot was aborted by the dispatcher, recycle it without executing the kernel
        // call. The dispatcher already returned an error to its caller, so applying side effects
        // for an abandoned request is incorrect.
        if scoreboard.slots[slot_index].state == ScoreBoardSlotState::Aborted {
            if let Err(error) = unsafe { scoreboard.available_slots.up() } {
                let reason: &str = "failed to recycle aborted scoreboard slot";
                warn!("{reason} (error={error:?})");
            }
            scoreboard.slots[slot_index].mark_free();
            return Err(Error::new(ErrorCode::TryAgain, "recycled aborted scoreboard slot"));
        }

        let args: &'static KcallArgs = &scoreboard.slots[slot_index].args;

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
    /// # Returns
    ///
    /// On successful completion, this function returns `Ok(())`. On failure, it returns an object
    /// that describes the error encountered.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The global scoreboard has not been initialized.
    /// - The slot index is out of bounds.
    /// - The slot is not in a state that accepts a result (neither `Signaled` nor `Aborted`).
    /// - Notifying the waiting dispatcher fails.
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
                if let Err(error) = unsafe { slot.handled.up() } {
                    // Roll back: clear the stored result and mark the slot as aborted so
                    // the next `handle()` iteration can recycle it and recover the
                    // available-slot semaphore count.
                    slot.mark_aborted();
                    let reason: &str = "failed to notify waiting dispatcher";
                    error!("{reason} (error={error:?})");
                    return Err(error);
                }

                Ok(())
            },
            ScoreBoardSlotState::Aborted => {
                // This arm handles a race between `handle()` and `handled()`: a slot can
                // transition to `Aborted` *after* `handle()` has already selected it (i.e.,
                // between the `is_pending()` check and the state comparison in `handle()`).
                // When the dispatcher's `Condvar::wait()` is interrupted it marks the slot as
                // `Aborted`, but the handler may have already dequeued the `pending_kcalls`
                // semaphore count for this slot and received it via `handle()`. In that
                // scenario `handled()` observes `Aborted` and must recycle the slot here.
                slot.result = None;

                // Release the slot back to the available pool. This is safe because when the
                // dispatcher aborted, it did not release the slot to ensure the handler could
                // process it exactly once. Semaphore::up() always increments the counter before
                // calling notify_first(), so even on failure the available slot count is already
                // restored. We mark the slot as free regardless.
                if let Err(error) = unsafe { scoreboard.available_slots.up() } {
                    let reason: &str = "failed to notify on available scoreboard slot recycle";
                    warn!("{reason} (error={error:?})");
                }

                slot.mark_free();

                Ok(())
            },
            _ => {
                let reason: &str = "slot is not in a signalable state";
                error!("{reason} (state={:?})", slot.state);
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
        }
    }

    ///
    /// # Description
    ///
    /// Searches for a free slot, fills it with kernel call arguments, and returns its index.
    ///
    /// # Parameters
    ///
    /// - `args`: Kernel call arguments describing the request.
    ///
    /// # Returns
    ///
    /// On success, returns `Some(index)` of the slot that was filled. Returns `None` if no free
    /// slot is available.
    ///
    fn find_free_slot(&mut self, args: KcallArgs) -> Option<usize> {
        let total_slots: usize = self.slots.len();
        let start_index: usize = self.next_dispatch_index;

        for offset in 0..total_slots {
            let current_index: usize = (start_index + offset) % total_slots;
            if self.slots[current_index].is_free() {
                self.slots[current_index].fill(args);
                self.next_dispatch_index = (current_index + 1) % total_slots;
                return Some(current_index);
            }
        }

        None
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScoreBoardSlotState {
    /// Slot is available for a new kernel call.
    Free,
    /// Transitional state: slot has been acquired and filled with arguments but not yet made
    /// visible to the handler. This state is never matched by `is_pending()` or `is_free()`,
    /// preventing the handler from observing a half-initialized slot.
    InUse,
    /// Slot has been signaled and is ready for the kernel to process.
    Signaled,
    /// Slot was abandoned by the dispatcher and requires reclamation by the handler.
    Aborted,
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
#[derive(Debug)]
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
    /// Semaphore used to wake the dispatcher when the kernel call is handled.
    handled: Semaphore,
}

impl ScoreBoardSlot {
    ///
    /// # Description
    ///
    /// Creates a scoreboard slot initialized with sentinel arguments.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of a scoreboard slot.
    ///
    fn new() -> Self {
        ScoreBoardSlot {
            state: ScoreBoardSlotState::Free,
            args: KcallArgs {
                pid: ProcessIdentifier::from(i32::MAX),
                tid: ThreadIdentifier::from(i32::MAX),
                number: 0,
                arg0: 0,
                arg1: 0,
                arg2: 0,
                arg3: 0,
            },
            result: None,
            handled: Semaphore::new(0),
        }
    }

    ///
    /// # Description
    ///
    /// Checks whether the slot is currently free.
    ///
    /// # Returns
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
    /// This predicate matches both `Signaled` and `Aborted` states because `handle()` needs to
    /// discover aborted slots to recycle them. After `is_pending()` returns `true`, `handle()`
    /// performs a second state check to distinguish `Signaled` from `Aborted` and either
    /// dispatches the kernel call or recycles the slot, respectively.
    ///
    /// The `result.is_none()` guard is required because `handled()` stores the result into the
    /// slot *before* waking the dispatcher via `notify_first()`. If the handler loop re-enters
    /// `handle()` before the dispatcher wakes, clears the result, and calls `mark_free()`, the
    /// slot is still in `Signaled` state with `result.is_some()`. Without this guard the handler
    /// would re-pick the same slot and attempt to process it a second time.
    ///
    /// The guard is always satisfied for `Aborted` slots because [`mark_aborted()`](Self::mark_aborted)
    /// unconditionally clears `result` to `None`. See the invariant documented on that method.
    ///
    /// # Returns
    ///
    /// True if the slot is pending processing, false otherwise.
    ///
    fn is_pending(&self) -> bool {
        matches!(self.state, ScoreBoardSlotState::Signaled | ScoreBoardSlotState::Aborted)
            && self.result.is_none()
    }

    ///
    /// # Description
    ///
    /// Fills the slot with kernel call arguments and marks it as in use.
    ///
    /// # Parameters
    ///
    /// - `args`: Kernel call arguments describing the request.
    ///
    fn fill(&mut self, args: KcallArgs) {
        self.state = ScoreBoardSlotState::InUse;
        self.args = args;
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
    /// # Invariant
    ///
    /// This method must always clear `result` to `None`. The [`is_pending()`](Self::is_pending)
    /// predicate relies on `result.is_none()` to distinguish genuinely pending slots from slots
    /// that have already been handled but not yet freed. If `result` were left as `Some`, the
    /// handler would skip the aborted slot entirely.
    ///
    fn mark_aborted(&mut self) {
        self.state = ScoreBoardSlotState::Aborted;
        self.result = None;
    }

    ///
    /// # Description
    ///
    /// Marks the slot as visible to the kernel thread. Must be called before signaling the
    /// pending semaphore so the handler never observes the slot in `InUse` state.
    ///
    fn mark_signaled(&mut self) {
        self.state = ScoreBoardSlotState::Signaled;
    }
}
