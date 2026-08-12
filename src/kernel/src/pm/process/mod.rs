// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::event::ProcessTerminationInfo;

//==================================================================================================
// Modules
//==================================================================================================

mod capability;
mod manager;
pub(crate) mod state;

//==================================================================================================
// Lifecycle Capacity Credits
//==================================================================================================

/// Reservation for a process-creation record and its future process-termination record.
#[derive(Debug)]
#[must_use]
pub(super) struct LifecycleCreationReservation {
    _private: (),
}

impl LifecycleCreationReservation {
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Capacity credit reserved for the future termination of a live process.
#[derive(Debug)]
#[must_use]
pub(super) struct LifecycleTerminationCredit {
    _private: (),
}

impl LifecycleTerminationCredit {
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Harvested process termination awaiting either lifecycle commit or explicit release.
#[must_use]
pub(crate) struct HarvestedProcess {
    info: ProcessTerminationInfo,
    termination_credit: LifecycleTerminationCredit,
}

impl HarvestedProcess {
    fn new(info: ProcessTerminationInfo, termination_credit: LifecycleTerminationCredit) -> Self {
        Self {
            info,
            termination_credit,
        }
    }

    /// Returns the harvested process-termination information.
    pub(crate) fn info(&self) -> &ProcessTerminationInfo {
        &self.info
    }

    fn into_parts(self) -> (ProcessTerminationInfo, LifecycleTerminationCredit) {
        (self.info, self.termination_credit)
    }
}

//==================================================================================================
// Exports
//==================================================================================================

pub use manager::{
    ExceptionGuard,
    ProcessManager,
    SigReturnFailure,
    SignalDeliveryOutcome,
    SleepError,
    SyncSignalOutcome,
};
pub use state::exception_to_signal;

//==================================================================================================
// Tests
//==================================================================================================

/// Runs all in-kernel unit tests for the process module.
#[cfg(feature = "test")]
pub(super) fn test() -> bool {
    let mut passed: bool = true;
    passed &= manager::test();
    passed &= state::test();
    passed
}
