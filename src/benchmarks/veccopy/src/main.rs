// Copyright(c) The Maintainers of Nanvix.
// Licensed by the MIT License.

#![no_std]
#![no_main]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;
use config::memory_layout::USER_HEAP_BASE_RAW;
use nvx::{mm::{AccessPermission, Address, VirtualAddress}, pm::ProcessIdentifier};
use ::nvx::sys::error::Error;
use serde::Deserialize;
use serde_json::from_str;
use ::nvx::sys::arch::mem;
use fastrand::Rng;

//==================================================================================================
// Structs
//==================================================================================================

#[derive(Deserialize)]
pub struct Parameters {
    seed: u64
}

impl Default for Parameters {
    fn default() -> Self {
        Self { 
            seed: 32
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

    let mypid: ProcessIdentifier = ::nvx::pm::getpid().expect("failed to get process identifier");

    let ntimes: usize = (config::kernel::MEMORY_SIZE / 8) / mem::PAGE_SIZE;
    let mut rng: Rng = Rng::with_seed(param.seed);

    for _ in  0..ntimes {
        let offset: usize = rng.usize(0..ntimes) * mem::PAGE_SIZE;
        let vaddr: VirtualAddress = VirtualAddress::from_raw_value(USER_HEAP_BASE_RAW + offset);
        
        match ::nvx::mm::mmap(mypid, vaddr, AccessPermission::RDWR) {
            Ok(_) => (),
            Err(_) => ::nvx::error!("failed to map page in address {:?}", vaddr),
        };  

        // Touch a byte inside the page to ensure it's mapped
        unsafe {
            let ptr: *mut u8 = vaddr.as_mut_ptr();
            core::ptr::write_volatile(ptr, rng.u8(..)); 
        }

        match ::nvx::mm::munmap(mypid, vaddr) {
            Ok(_) => (),
            Err(_) => ::nvx::error!("failed to unmap page in address {:?}", vaddr),
        }; 

    } 
    Ok(())
}
