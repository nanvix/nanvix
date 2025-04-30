// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod chdir;
mod close;
mod faccessat;
mod fchdir;
mod fchown;
mod fchownat;
mod fdatasync;
mod fsync;
mod ftruncate;
mod getcwd;
mod getids;
mod linkat;
mod lseek;
mod pipe;
mod pread;
mod pwrite;
mod read;
mod readlinkat;
mod symlinkat;
mod write;

//==================================================================================================
// Exports
//==================================================================================================

pub use self::{
    chdir::{
        ChangeDirectoryRequest,
        ChangeDirectoryResponse,
    },
    close::{
        CloseRequest,
        CloseResponse,
    },
    faccessat::{
        FileAccessAtRequest,
        FileAccessAtResponse,
    },
    fchdir::{
        FileChdirRequest,
        FileChdirResponse,
    },
    fchown::{
        FileChownRequest,
        FileChownResponse,
    },
    fchownat::{
        FileChownAtRequest,
        FileChownAtResponse,
    },
    fdatasync::{
        FileDataSyncRequest,
        FileDataSyncResponse,
    },
    fsync::{
        FileSyncRequest,
        FileSyncResponse,
    },
    ftruncate::{
        FileTruncateRequest,
        FileTruncateResponse,
    },
    getcwd::{
        GetCurrentWorkingDirectoryRequest,
        GetCurrentWorkingDirectoryResponse,
    },
    getids::{
        GetIdsRequest,
        GetIdsResponse,
    },
    linkat::{
        LinkAtRequest,
        LinkAtResponse,
    },
    lseek::{
        SeekRequest,
        SeekResponse,
    },
    pipe::{
        PipeRequest,
        PipeResponse,
    },
    pread::{
        PartialReadRequest,
        PartialReadResponse,
    },
    pwrite::{
        PartialWriteRequest,
        PartialWriteResponse,
    },
    read::{
        ReadRequest,
        ReadResponse,
    },
    readlinkat::{
        ReadLinkAtRequest,
        ReadLinkAtResponse,
    },
    symlinkat::{
        SymbolicLinkAtRequest,
        SymbolicLinkAtResponse,
    },
    write::{
        WriteRequest,
        WriteResponse,
    },
};
