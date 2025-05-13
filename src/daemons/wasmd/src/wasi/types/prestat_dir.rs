// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory::WriteBytes,
    wasi::Size,
};

//==================================================================================================
// Structures
//==================================================================================================

/// The contents of a [`Prestat`] when [`PreopenType::Dir`] is used.
#[repr(C)]
#[derive(Debug)]
pub struct PrestatDir {
    pr_name_len: Size,
}
::static_assert::assert_eq_align!(PrestatDir, 4);
::static_assert::assert_eq_size!(PrestatDir, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl PrestatDir {
    pub fn new(pr_name_len: Size) -> Self {
        Self { pr_name_len }
    }
}

impl WriteBytes for PrestatDir {
    fn write_le_bytes(&self, to: &mut [u8]) {
        self.pr_name_len.write_le_bytes(to);
    }
}
