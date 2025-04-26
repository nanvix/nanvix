// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod fcntl;
mod open;
mod openat;
mod renameat;
mod unlinkat;
mod utimensat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fadvise::posix_fadvise,
    fallocate::posix_fallocate,
    fcntl::fcntl,
    open::open,
    openat::openat,
    renameat::renameat,
    unlinkat::unlinkat,
    utimensat::utimensat,
};
