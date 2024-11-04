// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod openat;
mod renameat;
mod symlinkat;
mod unlinkat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fadvise::posix_fadvise,
    fallocate::posix_fallocate,
    openat::openat,
    renameat::renameat,
    symlinkat::symlinkat,
    unlinkat::unlinkat,
};
