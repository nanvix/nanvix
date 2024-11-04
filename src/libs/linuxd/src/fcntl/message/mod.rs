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
    fadvise::{
        FileAdvisoryInformationRequest,
        FileAdvisoryInformationResponse,
    },
    fallocate::{
        FileSpaceControlRequest,
        FileSpaceControlResponse,
    },
    openat::{
        OpenAtRequest,
        OpenAtResponse,
    },
    renameat::{
        RenameAtRequest,
        RenameAtResponse,
    },
    symlinkat::{
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
    unlinkat::{
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
};
