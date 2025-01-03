// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod capability;
mod identity;

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Tests kernel calls in the process management facility.
///
pub fn test() {
    identity::test();
    capability::test();
}
