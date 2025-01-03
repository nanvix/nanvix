// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::slice;
use ::hyperlight_common::flatbuffer_wrappers::function_types::ReturnValue;
use ::sys::error::{
    Error,
    ErrorCode,
};
use alloc::vec::Vec;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct InputData {
    pub inputDataSize: u64,
    pub inputDataBuffer: u64,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl InputData {
    // Pops the top element from the shared input data buffer and returns it as a T
    pub fn read<T>(&self) -> Result<T, Error>
    where
        T: for<'a> TryFrom<&'a [u8]>,
    {
        let shared_buffer_size = self.inputDataSize as usize;

        let idb = unsafe {
            slice::from_raw_parts_mut(self.inputDataBuffer as *mut u8, shared_buffer_size)
        };

        if idb.is_empty() {
            unimplemented!()
        }

        // get relative offset to next free address
        let stack_ptr_rel: u64 = u64::from_le_bytes(match idb[..8].try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(Error::new(ErrorCode::IoErr, "shared input buffer too small"));
            },
        });

        if stack_ptr_rel as usize > shared_buffer_size || stack_ptr_rel < 16 {
            unimplemented!()
        }

        // go back 8 bytes and read. This is the offset to the element on top of stack
        let last_element_offset_rel = u64::from_le_bytes(
            match idb[stack_ptr_rel as usize - 8..stack_ptr_rel as usize].try_into() {
                Ok(bytes) => bytes,
                Err(_) => {
                    return Err(Error::new(
                        ErrorCode::IoErr,
                        "Invalid stack pointer in pop_shared_input_data_into",
                    ));
                },
            },
        );

        let buffer = &idb[last_element_offset_rel as usize..];

        // convert the buffer to T
        let type_t = match T::try_from(buffer) {
            Ok(t) => Ok(t),
            Err(_e) => {
                unimplemented!()
            },
        };

        // update the stack pointer to point to the element we just popped of since that is now free
        idb[..8].copy_from_slice(&last_element_offset_rel.to_le_bytes());

        // zero out popped off buffer
        idb[last_element_offset_rel as usize..stack_ptr_rel as usize].fill(0);

        type_t
    }

    pub fn get_host_value_return_as_int(&self) -> Result<i32, Error> {
        let return_value = self.read::<ReturnValue>()?;

        // check that return value is an int and return
        if let ReturnValue::Int(i) = return_value {
            Ok(i)
        } else {
            unimplemented!();
        }
    }

    pub fn get_host_value_return_as_vecbytes(&self) -> Result<Vec<u8>, Error> {
        let return_value = self.read::<ReturnValue>()?;

        // check that return value is an Vec<u8> and return
        if let ReturnValue::VecBytes(v) = return_value {
            Ok(v)
        } else {
            unimplemented!("{:?}", return_value);
        }
    }
}
