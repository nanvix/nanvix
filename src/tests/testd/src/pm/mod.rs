// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod capability;
mod duplicate_burst;
mod sched;
mod terminate;

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    sched::test();
    capability::test();
    duplicate_burst::test();
    terminate::test();
}
