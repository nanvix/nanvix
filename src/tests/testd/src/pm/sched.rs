// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Tests for sched_yield()
//==================================================================================================

///
/// # Description
///
/// Tests if [`::nvx::pm::sched_yield`] works.
///
/// # Returns
///
/// If the test passed, `true` is returned. Otherwise, `false` is returned instead.
///
fn test_sched_yield() -> bool {
    matches!(::sys::kcall::sched::__kcall_sched_yield(), Ok(()))
}

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    crate::test!(test_sched_yield());
}
