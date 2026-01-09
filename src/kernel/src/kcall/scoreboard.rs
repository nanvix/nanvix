// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#![allow(dead_code)]
#![allow(unused_variables)]

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    KcallArgs,
    KcallResult,
};
use crate::pm::{
    sync::{
        condvar::Condvar,
        semaphore::Semaphore,
    },
    SleepError,
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
// Statistics
//==================================================================================================

///
/// # Description
///
/// Global scoreboard used to coordinate kernel call dispatching and completion signaling between
/// user and kernel threads.
///
static mut SCOREBOARD: Option<NewScoreBoard> = None;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Identifies a slot within the scoreboard storage.
///
pub struct ScoreBoardSlotIndex(usize);

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

///
/// # Description
///
/// Tracks the arguments, result, and synchronization primitive for a single slot.
///
struct NewScoreBoardSlot {
    state: ScoreBoardSlotState,
    args: KcallArgs,
    result: Option<KcallResult>,
    handled: Condvar,
}

///
/// # Description
///
/// Coordinates kernel call dispatching and completion signaling between user and kernel threads.
///
///
/// # Description
///
/// Coordinates kernel call dispatching and completion signaling across multiple slots.
///
pub struct NewScoreBoard {
    dispachers: Semaphore,
    dispatched: Semaphore,
    slots: [NewScoreBoardSlot; SCOREBOARD_SLOTS],
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NewScoreBoardSlot {
    ///
    /// # Description
    ///
    /// Creates a scoreboard slot initialized with sentinel arguments.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the returned slot participates in global scoreboard
    /// mutations that rely on interrupts being disabled instead of explicit locking.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts are disabled while the caller accesses the scoreboard slot.
    ///
    unsafe fn new() -> Self {
        NewScoreBoardSlot {
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
    /// # Safety
    ///
    /// This function is unsafe because it inspects slot state that is only synchronized by the
    /// interrupt-disabled execution context described for the scoreboard.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled for the duration of the check.
    ///
    unsafe fn is_free(&self) -> bool {
        matches!(self.state, ScoreBoardSlotState::Free)
    }

    ///
    /// # Description
    ///
    /// Checks whether the slot is pending processing.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it reads slot fields without additional locking, relying
    /// on interrupts being disabled for mutual exclusion.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled for the duration of the check.
    ///
    unsafe fn is_pending(&self) -> bool {
        matches!(self.state, ScoreBoardSlotState::InUse) && self.result.is_none()
    }

    ///
    /// # Description
    ///
    /// Fills the slot with kernel call arguments and marks it as in use.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates shared slot state that depends on interrupts
    /// being disabled to guarantee exclusivity.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled while mutating the slot.
    ///
    unsafe fn fill(
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
    unsafe fn as_usize(&self) -> usize {
        self.0
    }
}

impl NewScoreBoard {
    ///
    /// # Description
    ///
    /// Initializes the global scoreboard with empty slots and ready semaphores.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it mutates the global scoreboard without explicit locking,
    /// relying on interrupts being disabled to prevent concurrent access.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled while initializing the scoreboard.
    ///
    pub(crate) unsafe fn init() {
        SCOREBOARD = Some(NewScoreBoard {
            dispachers: Semaphore::new(SCOREBOARD_SLOTS),
            dispatched: Semaphore::new(0),
            slots: ::core::array::from_fn(|_| unsafe { NewScoreBoardSlot::new() }),
        });
    }

    ///
    /// # Description
    ///
    /// Fetches a mutable reference to the global scoreboard instance.
    ///
    /// # Errors
    ///
    /// Returns `ErrorCode::TryAgain` if the scoreboard was not initialized prior to use.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it provides mutable access to the global scoreboard without
    /// synchronization, which is only sound when interrupts are disabled.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled for the duration of the returned reference.
    ///
    unsafe fn get_mut() -> Result<&'static mut NewScoreBoard, Error> {
        if let Some(scoreboard) = SCOREBOARD.as_mut() {
            Ok(scoreboard)
        } else {
            let reason: &str = "uninitialized scoreboard";
            error!("{reason}");
            Err(Error::new(ErrorCode::TryAgain, reason))
        }
    }

    #[allow(clippy::too_many_arguments)]
    ///
    /// # Description
    ///
    /// Dispatches a kernel call into a free slot and waits for the handler to produce a result.
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
    /// Returns the `KcallResult` produced by the handler when successful.
    ///
    /// # Errors
    ///
    /// Propagates `SleepError` when slot allocation, semaphore, or condvar waits fail.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it manipulates the global scoreboard without explicit
    /// locking, assuming interrupts remain disabled across the kernel call path.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled until the dispatcher observes the handler result.
    /// - The caller does not hold kernel resources while waiting on the slot's condition variable.
    ///
    #[allow(clippy::too_many_arguments)]
    pub(crate) unsafe fn dispatch(
        number: u32,
        pid: ProcessIdentifier,
        tid: ThreadIdentifier,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
    ) -> Result<KcallResult, SleepError> {
        let scoreboard: &'static mut NewScoreBoard =
            unsafe { Self::get_mut() }.map_err(SleepError::Generic)?;

        unsafe { scoreboard.dispachers.down()? };

        let slot_index: usize = match scoreboard
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| unsafe { slot.is_free() })
        {
            Some((index, slot)) => {
                unsafe { slot.fill(number, pid, tid, arg0, arg1, arg2, arg3) };
                index
            },
            None => {
                let reason: &str = "no free scoreboard slots";
                error!("{reason}");
                unsafe { scoreboard.dispachers.up().map_err(SleepError::Generic)? };
                return Err(SleepError::Generic(Error::new(ErrorCode::TryAgain, reason)));
            },
        };

        unsafe { scoreboard.dispatched.up().map_err(SleepError::Generic)? };

        {
            let handled: &Condvar = &scoreboard.slots[slot_index].handled;
            unsafe { handled.wait(None)? };
        }

        let slot: &mut NewScoreBoardSlot = &mut scoreboard.slots[slot_index];
        let result: Option<KcallResult> = slot.result.take();
        slot.state = ScoreBoardSlotState::Free;

        unsafe { scoreboard.dispachers.up().map_err(SleepError::Generic)? };

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
    /// Returns the slot index paired with its immutable `KcallArgs` view.
    ///
    /// # Errors
    ///
    /// Returns `ErrorCode::TryAgain` when no dispatched slot is available after the semaphore is
    /// signaled.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it scans and mutates the global scoreboard without locks,
    /// assuming interrupts remain disabled while selecting a slot.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - Interrupts remain disabled until the handler finishes consuming the slot.
    ///
    pub(crate) unsafe fn handle() -> Result<(ScoreBoardSlotIndex, &'static KcallArgs), Error> {
        let scoreboard: &'static mut NewScoreBoard = unsafe { Self::get_mut() }?;

        scoreboard.dispatched.try_down()?;

        let (slot_index, args_ptr): (usize, *const KcallArgs) = match scoreboard
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| unsafe { slot.is_pending() })
        {
            Some((index, slot)) => (index, &slot.args as *const KcallArgs),
            None => {
                let reason: &str = "no dispatched slot available";
                error!("{reason}");
                unsafe { scoreboard.dispatched.up()? };
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
    /// # Errors
    ///
    /// Returns `ErrorCode::InvalidArgument` if the slot index is out of range and
    /// `ErrorCode::TryAgain` if the slot is not marked as in use.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it writes the handler result into the global scoreboard
    /// without locking, relying on interrupts remaining disabled while the slot is manipulated.
    ///
    /// It is safe to call this function if and only if the following conditions are met:
    /// - The slot index originated from `handle()` and has not been reused elsewhere.
    /// - Interrupts remain disabled while storing the result and signaling the dispatcher.
    ///
    pub(crate) unsafe fn handled(
        index: ScoreBoardSlotIndex,
        ret: KcallResult,
    ) -> Result<(), Error> {
        let scoreboard: &'static mut NewScoreBoard = unsafe { Self::get_mut() }?;

        let slot: &mut NewScoreBoardSlot = scoreboard
            .slots
            .get_mut(unsafe { index.as_usize() })
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument, "invalid slot index"))?;

        if !matches!(slot.state, ScoreBoardSlotState::InUse) {
            let reason: &str = "slot is not in use";
            error!("{reason}");
            return Err(Error::new(ErrorCode::TryAgain, reason));
        }

        slot.result = Some(ret);

        unsafe { slot.handled.notify_first()? };

        Ok(())
    }
}
