// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod aligned;
mod frame;
mod pd;
#[cfg(target_arch = "x86_64")]
mod pdpt;
mod pg;
mod phys;
#[cfg(target_arch = "x86_64")]
mod pml4;
mod pt;

#[cfg(test)]
mod test;

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sys::mm::{
    Address,
    VirtualAddress,
};
pub use aligned::*;
pub use frame::*;
#[allow(unused_imports)]
pub use pd::*;
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
pub use pdpt::*;
pub use pg::*;
pub use phys::*;
#[cfg(target_arch = "x86_64")]
pub use pml4::*;
#[allow(unused_imports)]
pub use pt::*;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(test)]
pub fn test() -> bool {
    let mut passed = true;

    passed &= test::test();

    passed
}
