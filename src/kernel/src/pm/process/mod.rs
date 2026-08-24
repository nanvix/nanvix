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

/// Capacity credit owned by a queued process-creation record. Reservations are counted rather than
/// identified, so this type carries no payload: it exists to make the credit a linear resource that
/// only the delivery broker can mint and consume.
#[derive(Debug)]
#[must_use]
pub(super) struct LifecycleCreationCredit {
    _private: (),
}

impl LifecycleCreationCredit {
    ///
    /// # Description
    ///
    /// Creates a creation-capacity credit.
    ///
    /// # Returns
    ///
    /// A creation-capacity credit.
    ///
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Reservation for a process-creation record and its future process-termination record.
#[derive(Debug)]
#[must_use]
pub(super) struct LifecycleCreationReservation {
    creation_credit: LifecycleCreationCredit,
    termination_credit: LifecycleTerminationCredit,
}

impl LifecycleCreationReservation {
    ///
    /// # Description
    ///
    /// Creates a reservation that owns one creation credit and one termination credit.
    ///
    /// # Returns
    ///
    /// A creation reservation.
    ///
    fn new() -> Self {
        Self {
            creation_credit: LifecycleCreationCredit::new(),
            termination_credit: LifecycleTerminationCredit::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Splits the reservation into its creation and termination credits.
    ///
    /// # Returns
    ///
    /// A tuple holding the creation credit and the termination credit.
    ///
    fn into_credits(self) -> (LifecycleCreationCredit, LifecycleTerminationCredit) {
        (self.creation_credit, self.termination_credit)
    }
}

/// Capacity credit reserved for the future termination of a live process.
#[derive(Debug)]
#[must_use]
pub(super) struct LifecycleTerminationCredit {
    _private: (),
}

impl LifecycleTerminationCredit {
    ///
    /// # Description
    ///
    /// Creates a termination-capacity credit.
    ///
    /// # Returns
    ///
    /// A termination-capacity credit.
    ///
    fn new() -> Self {
        Self { _private: () }
    }
}

/// Capacity credit reserved for the future termination of a live thread.
#[derive(Debug)]
#[must_use]
pub(crate) struct ThreadLifecycleTerminationCredit {
    _private: (),
}

/// Reservation for the future termination record of a thread being created.
#[derive(Debug)]
#[must_use]
pub(super) struct ThreadLifecycleReservation {
    termination_credit: ThreadLifecycleTerminationCredit,
}

impl ThreadLifecycleReservation {
    ///
    /// # Description
    ///
    /// Creates a thread lifecycle reservation.
    ///
    /// # Returns
    ///
    /// A thread lifecycle reservation.
    ///
    fn new() -> Self {
        Self {
            termination_credit: ThreadLifecycleTerminationCredit { _private: () },
        }
    }

    ///
    /// # Description
    ///
    /// Converts this reservation into the credit owned by a committed thread.
    ///
    /// # Returns
    ///
    /// A thread-termination capacity credit.
    ///
    fn into_credit(self) -> ThreadLifecycleTerminationCredit {
        self.termination_credit
    }
}

/// Process termination metadata retained through resource harvest.
#[must_use]
pub(crate) struct HarvestedProcess {
    info: ProcessTerminationInfo,
}

impl HarvestedProcess {
    ///
    /// # Description
    ///
    /// Creates harvested process metadata from a process-termination record.
    ///
    /// # Parameters
    ///
    /// - `info`: Process-termination record retained through resource harvest.
    ///
    /// # Returns
    ///
    /// Harvested process metadata containing `info`.
    ///
    fn new(info: ProcessTerminationInfo) -> Self {
        Self { info }
    }

    /// Returns the harvested process-termination information.
    pub(crate) fn info(&self) -> &ProcessTerminationInfo {
        &self.info
    }

    ///
    /// # Description
    ///
    /// Consumes this harvested process and returns its termination information.
    ///
    /// # Returns
    ///
    /// The harvested process-termination information.
    ///
    pub(crate) fn into_info(self) -> ProcessTerminationInfo {
        self.info
    }
}

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "test")]
pub(crate) use manager::new_test_delivery_sequence;
///
/// # Description
///
/// Creates a synthetic thread-termination credit for an in-kernel test fixture.
///
/// # Returns
///
/// A synthetic thread-termination credit.
///
#[cfg(feature = "test")]
pub(crate) fn new_test_thread_termination_credit() -> ThreadLifecycleTerminationCredit {
    ThreadLifecycleTerminationCredit { _private: () }
}
///
/// # Description
///
/// Creates a synthetic process-termination credit for an in-kernel test fixture.
///
/// # Returns
///
/// A synthetic process-termination credit.
///
#[cfg(feature = "test")]
pub(crate) fn new_test_process_termination_credit() -> LifecycleTerminationCredit {
    LifecycleTerminationCredit { _private: () }
}
pub(crate) use manager::DeliverySequence;
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
