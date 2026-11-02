// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    manager,
    state,
    LifecycleTerminationCredit,
    ThreadLifecycleTerminationCredit,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a synthetic thread-termination credit for an in-kernel test fixture.
///
/// # Returns
///
/// A synthetic thread-termination credit.
///
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
pub(crate) fn new_test_process_termination_credit() -> LifecycleTerminationCredit {
    LifecycleTerminationCredit { _private: () }
}

/// Runs all in-kernel unit tests for the process module.
pub(in crate::pm) fn test() -> bool {
    let mut passed: bool = true;
    passed &= manager::test();
    passed &= state::test();
    passed
}
