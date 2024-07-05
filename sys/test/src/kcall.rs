/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==============================================================================
// Imports
//==============================================================================

use nanvix::kcall;

//==============================================================================
// Private Standalone Functions
//==============================================================================

/// Issues a void0 kernel call.
fn issue_void0_kcall() -> bool {
    kcall::void0() == 0
}

/// Issues a void1 kernel call.
fn issue_void1_kcall() -> bool {
    kcall::void1(1) == 1
}

/// Issues a void2 kernel call.
fn issue_void2_kcall() -> bool {
    kcall::void2(1, 2) == 3
}

/// Issues a void3 kernel call.
fn issue_void3_kcall() -> bool {
    kcall::void3(1, 2, 3) == 6
}

/// Issues a void4 kernel call.
fn issue_void4_kcall() -> bool {
    kcall::void4(1, 2, 3, 4) == 10
}

//==============================================================================
// Public Standalone Functions
//==============================================================================

///
/// **Description**
///
/// Tests if we can issue kernel calls.
///
pub fn test() {
    crate::test!(issue_void0_kcall());
    crate::test!(issue_void1_kcall());
    crate::test!(issue_void2_kcall());
    crate::test!(issue_void3_kcall());
    crate::test!(issue_void4_kcall());
}
