// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory::WriteBytes,
    wasi::{
        PrestatDir,
        Size,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

/// Information about a pre-opened capability.
#[derive(Debug)]
#[repr(C, align(4))]
pub struct Prestat {
    padding: [u8; Self::SIZE_OF_PADDING],
    dir: PrestatDir,
}
::nvx::sys::static_assert_alignment!(Prestat, Prestat::_ALIGNMENT_OF_PRESTAT);
::nvx::sys::static_assert_size!(Prestat, Prestat::_SIZE_OF_PRESTAT);

//==================================================================================================
// Implementations
//==================================================================================================

impl Prestat {
    const _ALIGNMENT_OF_PRESTAT: usize = 4;
    const _SIZE_OF_PRESTAT: usize = 8;
    const OFFSET_OF_PADDING: usize = 0;
    const OFFSET_OF_DIR: usize = 4;
    const SIZE_OF_PADDING: usize = 4;
    const SIZE_OF_DIR: usize = 4;

    pub fn new(pr_name_len: Size) -> Self {
        Self {
            padding: [0; Self::SIZE_OF_PADDING],
            dir: PrestatDir::new(pr_name_len),
        }
    }
}

impl WriteBytes for Prestat {
    fn write_le_bytes(&self, to: &mut [u8]) {
        self.padding.write_le_bytes(
            &mut to[Self::OFFSET_OF_PADDING..Self::OFFSET_OF_PADDING + Self::SIZE_OF_PADDING],
        );
        self.dir
            .write_le_bytes(&mut to[Self::OFFSET_OF_DIR..Self::OFFSET_OF_DIR + Self::SIZE_OF_DIR]);
    }
}
