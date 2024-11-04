// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod openat;
mod readlinkat;
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
    readlinkat::readlinkat,
    renameat::renameat,
    symlinkat::symlinkat,
    unlinkat::unlinkat,
};
