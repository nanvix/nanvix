// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod chdir;
mod close;
mod dup2;
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
mod register_socket;
mod resolve_fd;
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
    dup2::{
        Dup2Request,
        Dup2Response,
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
    register_socket::{
        RegisterSocketRequest,
        RegisterSocketResponse,
    },
    resolve_fd::{
        ResolveFdRequest,
        ResolveFdResponse,
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
