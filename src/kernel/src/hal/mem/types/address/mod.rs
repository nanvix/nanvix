// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod aligned;
mod frame;
mod page;
mod pd;
#[cfg(target_arch = "x86_64")]
mod pdpt;
mod phys;
#[cfg(target_arch = "x86_64")]
mod pml4;
mod pt;

#[cfg(feature = "test")]
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
pub use page::*;
#[allow(unused_imports)]
pub use pd::*;
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
pub use pdpt::*;
pub use phys::*;
#[cfg(target_arch = "x86_64")]
#[allow(unused_imports)]
pub use pml4::*;
#[allow(unused_imports)]
pub use pt::*;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(feature = "test")]
pub fn test() -> bool {
    let mut passed = true;

    passed &= test::test();

    passed
}
