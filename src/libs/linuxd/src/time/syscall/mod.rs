// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod clock_getres;
mod clock_gettime;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    clock_getres::clock_getres,
    clock_gettime::clock_gettime,
};
