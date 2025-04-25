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
mod openat;
mod readlinkat;
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
    fchmodat::{
        FileChmodAtRequest,
        FileChmodAtResponse,
    },
    fchownat::{
        FileChownAtRequest,
        FileChownAtResponse,
    },
    fcntl::{
        FileControlRequest,
        FileControlResponse,
    },
    openat::{
        OpenAtRequest,
        OpenAtResponse,
    },
    readlinkat::{
        ReadLinkAtRequest,
        ReadLinkAtResponse,
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
