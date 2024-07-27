// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::cpu::madt::MadtEntryHeader;

//==================================================================================================
// Implementations
//==================================================================================================

impl MadtEntryHeader {
    pub unsafe fn from_ptr(ptr: *const MadtEntryHeader) -> Self {
        Self {
            typ: (*ptr).typ,
            len: (*ptr).len,
        }
    }
}
