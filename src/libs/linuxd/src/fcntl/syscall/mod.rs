// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fallocate;
mod openat;
mod renameat;
mod unlinkat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fallocate::posix_fallocate,
    openat::openat,
    renameat::renameat,
    unlinkat::unlinkat,
};
