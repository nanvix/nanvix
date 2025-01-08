// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod fchownat;
mod fcntl;
mod mkdirat;
mod open;
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
    fchownat::fchownat,
    fcntl::fcntl,
    mkdirat::mkdirat,
    open::open,
    openat::openat,
    readlinkat::readlinkat,
    renameat::renameat,
    symlinkat::symlinkat,
    unlinkat::unlinkat,
};
