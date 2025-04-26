// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod close;
mod fchdir;
mod fchown;
mod fdatasync;
mod fsync;
mod ftruncate;
mod getcwd;
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
    close::{
        CloseRequest,
        CloseResponse,
    },
    fchdir::{
        FileChdirRequest,
        FileChdirResponse,
    },
    fchown::{
        FileChownRequest,
        FileChownResponse,
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
