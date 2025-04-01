// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(never_type)] // pthread requires this.
#![feature(c_variadic)] // fcntl requires this.
#![feature(btree_extract_if)] // dlfcn requires this.

//==================================================================================================
// Modules
//==================================================================================================

extern crate alloc;

// Address and routing parameter area.
pub mod arpa;

/// Format of directory entries
pub mod dirent;

/// Dynamic linking.
pub mod dlfcn;

/// System error numbers.
pub mod errno;

/// Foreign function interface.
pub mod ffi;

/// Time types.
pub mod time;

/// Virtual environments.
pub mod venv;

/// File control operations.
pub mod fcntl;

/// Implementation-defined constants.
pub mod limits;

/// Messages.
pub mod message;

/// Internet protocols for network stack.
pub mod netinet;

/// Posix threads.
pub mod pthread;

/// Standard symbolic constants and types.
pub mod unistd;

/// File last access and modification times.
pub mod utime;

/// Execution scheduling.
pub mod sched;

/// Signals
pub mod signal;

/// System-specific headers.
pub mod sys;

// Safe wrappers.
#[cfg(feature = "syscall")]
mod safe;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    convert::TryFrom,
    mem,
};
use ::num_enum::TryFromPrimitive;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u16)]
pub enum LinuxDaemonMessageHeader {
    GetClockResolutionRequest,
    GetClockResolutionResponse,
    GetClockTimeRequest,
    GetClockTimeResponse,
    OpenAtRequest,
    OpenAtRequestPart,
    OpenAtResponse,
    UnlinkAtRequest,
    UnlinkAtResponse,
    CloseRequest,
    CloseResponse,
    RenameAtRequest,
    RenameAtResponse,
    FileStatAtRequestPart,
    FileStatAtResponsePart,
    FileDataSyncRequest,
    FileDataSyncResponse,
    FileSyncRequest,
    FileSyncResponse,
    SeekRequest,
    SeekResponse,
    FileSpaceControlRequest,
    FileSpaceControlResponse,
    FileTruncateRequest,
    FileTruncateResponse,
    FileAdvisoryInformationRequest,
    FileAdvisoryInformationResponse,
    FileStatRequest,
    FileStatResponse,
    WriteRequest,
    WriteResponse,
    ReadRequest,
    ReadResponse,
    PartialWriteRequest,
    PartialWriteResponse,
    PartialReadRequest,
    PartialReadResponse,
    SymbolicLinkAtRequestPart,
    SymbolicLinkAtResponse,
    LinkAtRequestPart,
    LinkAtResponse,
    ReadLinkAtRequestPart,
    ReadLinkAtResponsePart,
    MakeDirectoryAtRequestPart,
    MakeDirectoryAtResponse,
    UpdateFileAccessTimeAtRequestPart,
    UpdateFileAccessTimeAtResponse,
    UpdateFileAccessTimeRequest,
    UpdateFileAccessTimeResponse,
    FileControlRequest,
    FileControlResponse,
    CreateSocketRequest,
    CreateSocketResponse,
    BindSocketRequest,
    BindSocketResponse,
    ListenSocketRequest,
    ListenSocketResponse,
    AcceptSocketRequest,
    AcceptSocketResponse,
    ShutdownSocketRequest,
    ShutdownSocketResponse,
    ReceiveSocketRequest,
    ReceiveSocketResponse,
    SendSocketRequest,
    SendSocketResponse,
    TimesRequest,
    TimesResponse,
    FileChownAtRequestPart,
    FileChownAtResponse,
    FileChownRequest,
    FileChownResponse,
    FileChmodAtRequestPart,
    FileChmodAtResponse,
    FileChmodRequest,
    FileChmodResponse,
    ConnectSocketRequest,
    ConnectSocketResponse,
    CreateSocketPairRequest,
    CreateSocketPairResponse,
    GetPeerNameRequest,
    GetPeerNameResponse,
    GetSockNameRequest,
    GetSockNameResponse,
    PipeRequest,
    PipeResponse,
    GetCurrentWorkingDirectoryRequest,
    GetCurrentWorkingDirectoryResponse,
    GetCurrentWorkingDirectoryResponsePart,
    GetDirectoryEntriesRequest,
    GetDirectoryEntriesResponse,
    GetDirectoryEntriesResponsePart,
}

#[repr(C, packed)]
pub struct LinuxDaemonMessage {
    /// Message header.
    pub header: LinuxDaemonMessageHeader,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::nvx::sys::static_assert_size!(LinuxDaemonMessage, Message::PAYLOAD_SIZE);

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Process identifier of the Linux Daemon Service
///
pub const LINUXD: ProcessIdentifier = ProcessIdentifier::KERNEL;

//==================================================================================================
// Implementations
//==================================================================================================

impl LinuxDaemonMessage {
    pub const PAYLOAD_SIZE: usize =
        Message::PAYLOAD_SIZE - mem::size_of::<LinuxDaemonMessageHeader>();

    pub fn new(header: LinuxDaemonMessageHeader, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self { header, payload }
    }

    pub fn try_from_bytes(bytes: [u8; Message::PAYLOAD_SIZE]) -> Result<Self, Error> {
        // Check if message header is valid.
        let _header: LinuxDaemonMessageHeader =
            LinuxDaemonMessageHeader::try_from(u16::from_ne_bytes([bytes[0], bytes[1]]))
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid message header"))?;

        let message: LinuxDaemonMessage = unsafe { mem::transmute(bytes) };

        Ok(message)
    }

    pub fn into_bytes(self) -> [u8; Message::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}
