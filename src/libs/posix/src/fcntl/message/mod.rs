// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod fadvise;
mod fallocate;
mod fcntl;
mod openat;
mod renameat;
mod unlinkat;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    fadvise::{
        FileAdvisoryInformationRequest,
        FileAdvisoryInformationResponse,
    },
    fallocate::{
        FileSpaceControlRequest,
        FileSpaceControlResponse,
    },
    fcntl::{
        FileControlRequest,
        FileControlResponse,
    },
    openat::{
        OpenAtRequest,
        OpenAtResponse,
    },
    renameat::{
        RenameAtRequest,
        RenameAtResponse,
    },
    unlinkat::{
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
};
