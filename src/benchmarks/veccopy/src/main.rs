// Copyright(c) The Maintainers of Nanvix.
// Licensed by the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
use ::alloc::vec::Vec;
use ::nvx::sys::error::Error;
use serde::Deserialize;
use serde_json::from_str;

//==================================================================================================
// Structs
//==================================================================================================

#[derive(Deserialize)]
pub struct Parameters {
    vec_size: usize
}

impl Default for Parameters {
    fn default() -> Self {
        Self { 
            vec_size: 10000
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[no_mangle]
fn main() -> Result<(), Error> {
    let raw_content: Option<&str> = option_env!("CONFIG");

    let param: Parameters = if let Some(raw_content) = raw_content {
        from_str(raw_content).expect("failed to parse CONFIG environment variable")
    } else {
        Parameters::default()
    };

    let mut source: Vec<u32> = Vec::with_capacity(param.vec_size);  
    for i in 0..param.vec_size {
        source.push(i as u32); 
    }

    for i in 0..param.vec_size {
        source[i] = source[i] * 1; 
    }

    let mut dst: Vec<u32> = Vec::with_capacity(param.vec_size);
    unsafe {
        dst.set_len(param.vec_size);
    }

    // Perform memory copy
    unsafe {
        ::core::ptr::copy_nonoverlapping(source.as_ptr(), dst.as_mut_ptr(), param.vec_size);
    }

    Ok(())
}
