// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::core::{
    mem,
    slice,
};
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct OutputData {
    pub size: u64,
    pub buffer: u64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl OutputData {
    /// Size of a stack pointer in bytes.
    const STACK_PTR_SIZE: usize = mem::size_of::<u64>();

    pub fn write(&self, data: Vec<u8>) -> Result<(), Error> {
        let shared_buffer_size = self.size as usize;
        let odb: &mut [u8] =
            unsafe { slice::from_raw_parts_mut(self.buffer as *mut u8, shared_buffer_size) };

        if odb.len() < Self::STACK_PTR_SIZE {
            return Err(Error::new(ErrorCode::IoErr, "shared output buffer is too small"));
        }

        // get offset to next free address on the stack
        let stack_ptr_rel: u64 = u64::from_le_bytes(match odb[..Self::STACK_PTR_SIZE].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(Error::new(
                    ErrorCode::IoErr,
                    "failed to get stack pointer in shared output buffer",
                ));
            },
        });

        // check if the stack pointer is within the bounds of the buffer.
        // It can be equal to the size, but never greater
        // It can never be less than 8. An empty buffer's stack pointer is 8
        if stack_ptr_rel as usize > shared_buffer_size {
            return Err(Error::new(
                ErrorCode::IoErr,
                "invalid stack pointer in shared output buffer",
            ));
        }

        // check if there is enough space in the buffer
        let size_required: usize = data.len() + Self::STACK_PTR_SIZE; // the data plus the pointer pointing to the data
        let size_available: usize = shared_buffer_size - stack_ptr_rel as usize;
        if size_required > size_available {
            return Err(Error::new(ErrorCode::IoErr, "not enough space in shared output buffer"));
        }

        // write the actual data
        odb[stack_ptr_rel as usize..stack_ptr_rel as usize + data.len()].copy_from_slice(&data);

        // write the offset to the newly written data, to the top of the stack
        let bytes: [u8; Self::STACK_PTR_SIZE] = stack_ptr_rel.to_le_bytes();
        odb[stack_ptr_rel as usize + data.len()
            ..stack_ptr_rel as usize + data.len() + Self::STACK_PTR_SIZE]
            .copy_from_slice(&bytes);

        // update stack pointer to point to next free address
        let new_stack_ptr_rel: u64 =
            (stack_ptr_rel as usize + data.len() + Self::STACK_PTR_SIZE) as u64;
        odb[0..8].copy_from_slice(&(new_stack_ptr_rel).to_le_bytes());

        Ok(())
    }
}
