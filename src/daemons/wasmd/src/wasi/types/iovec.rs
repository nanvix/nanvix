// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    memory::{
        ReadBytes,
        ReadBytesError,
    },
    wasi::{
        types::Pointer,
        Size,
    },
};
use ::core::mem;

//==================================================================================================
// Structures
//==================================================================================================

/// A region of memory for scatter/gather writes.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IoVec {
    /// Base address of the buffer.
    buf: Pointer<u8>,
    /// Length of the buffer.
    buf_len: Size,
}
::static_assert::assert_eq_align!(IoVec, 4);
::static_assert::assert_eq_size!(IoVec, 8);

impl IoVec {
    /// Creates a new region of memory for scatter/gather writes.
    pub fn from_raw_parts(buf: Pointer<u8>, buf_len: Size) -> Self {
        Self { buf, buf_len }
    }

    /// Returns the base address of the buffer.
    pub fn buf(&self) -> Pointer<u8> {
        self.buf
    }

    /// Returns the length of the buffer.
    pub fn buf_len(&self) -> Size {
        self.buf_len
    }
}

impl ReadBytes for IoVec {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        Ok(Self {
            buf: Pointer::<u8>::read_le_bytes(from)?,
            buf_len: Size::read_le_bytes(&from[mem::size_of::<Pointer<u8>>()..])?,
        })
    }
}
