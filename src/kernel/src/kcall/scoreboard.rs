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
    // Synchronizes number slots available for kernel call dispatching.
    available_slots: Semaphore,
    // Synchronizes number of kernel calls ready to be handled.
    pending_kcalls: Semaphore,
    // Slots for kernel call dispatching.
    slots: [ScoreBoardSlot; SCOREBOARD_SLOTS],
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
        });
    }

    ///
    /// # Description
    ///
    /// Gets a mutable reference to the global scoreboard instance.
    ///
    /// # Return
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
    /// # Returns
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
        if let Err(error) = unsafe { scoreboard.pending_kcalls.up() } {
            // TODO: signal available_slots semaphore and free slot.

            let reason: &str = "failed to signal pending kernel calls";
            error!("{reason}");
            return Err(SleepError::Generic(error));
        }

        // Wait for the kernel call to be handled.
        {
            let handled: &Condvar = &scoreboard.slots[slot_index].handled;
            if let Err(error) = unsafe { handled.wait(None) } {
                // TODO: signal available_slots semaphore and free slot.

                let reason: &str = "failed to wait for kernel call handling";
                error!("{reason}");
                return Err(error);
            }
        }

        // Signal slot availability before freeing it; if signaling fails the slot is still marked
        // as in use, preventing other dispatchers from reusing it prematurely. This is safe because
        // the up() operation does not incur in a context switch.
        if let Err(error) = unsafe { scoreboard.available_slots.up() } {
            let reason: &str = "failed to signal available scoreboard slots";
            error!("{reason}");
            return Err(SleepError::Generic(error));
        }

        // Get the slot.
        let slot: &mut ScoreBoardSlot = &mut scoreboard.slots[slot_index];

        // Retrieve the result.
        let result: Option<KcallResult> = slot.result.take();

        // Free the slot.
        slot.state = ScoreBoardSlotState::Free;

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
    /// kernel arguments. On failure, it returns an object that describes the error
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
            let reason: &str = "no pending kernel calls";
            error!("{reason}");
            return Err(error);
        }

        // Find a pending slot.
        // TODO: restart from previous search index and cycle through slots to avoid starvation.
        let (slot_index, args_ptr): (usize, *const KcallArgs) = match scoreboard
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_pending())
        {
            Some((index, slot)) => (index, &slot.args as *const KcallArgs),
            None => {
                let reason: &str = "no dispatched slot available";
                error!("{reason}");
                unsafe { scoreboard.pending_kcalls.up()? };
                return Err(Error::new(ErrorCode::TryAgain, reason));
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

        // Verify that the slot is in use.
        if !matches!(slot.state, ScoreBoardSlotState::InUse) {
            let reason: &str = "slot is not in use";
            error!("{reason}");
            return Err(Error::new(ErrorCode::TryAgain, reason));
        }

        slot.result = Some(ret);

        // Notify the waiting dispatcher.
        if let Err(error) = unsafe { slot.handled.notify_first() } {
            let reason: &str = "failed to notify waiting dispatcher";
            error!("{reason}");
            return Err(error);
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
    Free,
    InUse,
}

//==================================================================================================
// Scoreboard Slot Index
//==================================================================================================

///
/// # Description
///
/// Identifies a slot within the scoreboard storage.
///
pub struct ScoreBoardSlotIndex(usize);

impl ScoreBoardSlotIndex {
    ///
    /// # Description
    ///
    /// Converts the slot index into a `usize` for array access.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned index is consumed by code that mutates the
    /// scoreboard without locking, relying on interrupts being disabled for exclusivity.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled while the returned index is used.
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
    state: ScoreBoardSlotState,
    args: KcallArgs,
    result: Option<KcallResult>,
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
        matches!(self.state, ScoreBoardSlotState::InUse) && self.result.is_none()
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
}
