// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod aligned;
mod frame;
mod page;
mod pd;

#[cfg(feature = "test")]
mod test;

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sys::mm::{
    Address,
    PhysicalAddress,
    VirtualAddress,
};
pub use aligned::*;
pub use frame::*;
pub use page::*;
pub use pd::*;
#[cfg(feature = "test")]
pub use test::test;
