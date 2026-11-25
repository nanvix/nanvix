// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    identity_map,
    vmem,
    Vmem,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all in-kernel tests of the virtual memory subsystem.
pub fn test(root: &Vmem) {
    assert!(identity_map::test::test());
    assert!(vmem::test::test(root));
}
