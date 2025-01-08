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
mod openat;
mod readlinkat;
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
    fchownat::{
        FileChownAtRequest,
        FileChownAtResponse,
    },
    fcntl::{
        FileControlRequest,
        FileControlResponse,
    },
    mkdirat::{
        MakeDirectoryAtRequest,
        MakeDirectoryAtResponse,
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
    symlinkat::{
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
    unlinkat::{
        UnlinkAtRequest,
        UnlinkAtResponse,
    },
};
