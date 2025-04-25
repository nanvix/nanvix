// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod fchmodat;
mod fchownat;
mod fcntl;
mod open;
mod openat;
mod readlinkat;
mod renameat;
mod unlinkat;
mod utimensat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fadvise::posix_fadvise,
    fallocate::posix_fallocate,
    fchmodat::fchmodat,
    fchownat::fchownat,
    fcntl::fcntl,
    open::open,
    openat::openat,
    readlinkat::readlinkat,
    renameat::renameat,
    unlinkat::unlinkat,
    utimensat::utimensat,
};
