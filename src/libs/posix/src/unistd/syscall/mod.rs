// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod close;
mod fdatasync;
mod fsync;
mod ftruncate;
mod link;
mod linkat;
mod lseek;
mod open;
mod pread;
mod pwrite;
mod read;
mod unlink;
mod write;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    close::close,
    fdatasync::fdatasync,
    fsync::fsync,
    ftruncate::ftruncate,
    link::link,
    linkat::linkat,
    lseek::lseek,
    open::open,
    pread::pread,
    pwrite::pwrite,
    read::read,
    unlink::unlink,
    write::write,
};
