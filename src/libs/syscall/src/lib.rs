// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![feature(never_type)] // pthread requires this.
#![feature(c_variadic)] // fcntl requires this.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

#[cfg(any(feature = "syscall", feature = "rustc-dep-of-std"))]
extern crate syslog;

#[cfg(any(feature = "rustc-dep-of-std", feature = "staticlib"))]
#[allow(unused_extern_crates)]
extern crate libc_stdlib;

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_extern_crates)]
extern crate nvx;

#[cfg(feature = "rustc-dep-of-std")]
pub use ::syslog;

#[cfg(feature = "rustc-dep-of-std")]
pub use ::sysapi;

#[cfg(feature = "rustc-dep-of-std")]
pub use ::sys::error;

#[cfg(feature = "rustc-dep-of-std")]
pub use ::sysalloc;

pub mod errno;

/// Definitions for internet operations.
pub mod arpa;

/// Format of directory entries
pub mod dirent;

/// Dynamic linking.
pub mod dlfcn;

/// Time types.
pub mod time;

/// Virtual environments.
pub mod venv;

/// File control operations.
pub mod fcntl;

/// Messages.
pub mod message;

/// Internet protocols for network stack.
pub mod netinet;

/// Posix threads.
pub mod pthread;

/// Standard symbolic constants and types.
pub mod unistd;

/// Execution scheduling.
pub mod sched;

/// Signals.
pub mod signal;

/// System-specific headers.
pub mod sys;

/// Definitions for I/O polling.
pub mod poll;

// Safe wrappers.
#[cfg(feature = "syscall")]
pub mod safe;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    convert::TryFrom,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ProcessIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum LinuxDaemonMessageHeader {
    OpenAtRequestPart,
    OpenAtResponse,
    UnlinkAtRequestPart,
    UnlinkAtResponse,
    CloseRequest,
    CloseResponse,
    RenameAtRequestPart,
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
    FileChdirRequest,
    FileChdirResponse,
    ChangeDirectoryRequestPart,
    ChangeDirectoryResponse,
    FileAccessAtRequestPart,
    FileAccessAtResponse,
    GetIdsRequest,
    GetIdsResponse,
    PollRequestPart,
    PollResponsePart,
    SelectRequest,
    SelectResponse,
}
// Manual TryFrom<u16> implementation for LinuxDaemonMessageHeader
impl TryFrom<u16> for LinuxDaemonMessageHeader {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        use LinuxDaemonMessageHeader::*;
        match value {
            x if x == OpenAtRequestPart as u16 => Ok(OpenAtRequestPart),
            x if x == OpenAtResponse as u16 => Ok(OpenAtResponse),
            x if x == UnlinkAtRequestPart as u16 => Ok(UnlinkAtRequestPart),
            x if x == UnlinkAtResponse as u16 => Ok(UnlinkAtResponse),
            x if x == CloseRequest as u16 => Ok(CloseRequest),
            x if x == CloseResponse as u16 => Ok(CloseResponse),
            x if x == RenameAtRequestPart as u16 => Ok(RenameAtRequestPart),
            x if x == RenameAtResponse as u16 => Ok(RenameAtResponse),
            x if x == FileStatAtRequestPart as u16 => Ok(FileStatAtRequestPart),
            x if x == FileStatAtResponsePart as u16 => Ok(FileStatAtResponsePart),
            x if x == FileDataSyncRequest as u16 => Ok(FileDataSyncRequest),
            x if x == FileDataSyncResponse as u16 => Ok(FileDataSyncResponse),
            x if x == FileSyncRequest as u16 => Ok(FileSyncRequest),
            x if x == FileSyncResponse as u16 => Ok(FileSyncResponse),
            x if x == SeekRequest as u16 => Ok(SeekRequest),
            x if x == SeekResponse as u16 => Ok(SeekResponse),
            x if x == FileSpaceControlRequest as u16 => Ok(FileSpaceControlRequest),
            x if x == FileSpaceControlResponse as u16 => Ok(FileSpaceControlResponse),
            x if x == FileTruncateRequest as u16 => Ok(FileTruncateRequest),
            x if x == FileTruncateResponse as u16 => Ok(FileTruncateResponse),
            x if x == FileAdvisoryInformationRequest as u16 => Ok(FileAdvisoryInformationRequest),
            x if x == FileAdvisoryInformationResponse as u16 => Ok(FileAdvisoryInformationResponse),
            x if x == FileStatRequest as u16 => Ok(FileStatRequest),
            x if x == FileStatResponse as u16 => Ok(FileStatResponse),
            x if x == WriteRequest as u16 => Ok(WriteRequest),
            x if x == WriteResponse as u16 => Ok(WriteResponse),
            x if x == ReadRequest as u16 => Ok(ReadRequest),
            x if x == ReadResponse as u16 => Ok(ReadResponse),
            x if x == PartialWriteRequest as u16 => Ok(PartialWriteRequest),
            x if x == PartialWriteResponse as u16 => Ok(PartialWriteResponse),
            x if x == PartialReadRequest as u16 => Ok(PartialReadRequest),
            x if x == PartialReadResponse as u16 => Ok(PartialReadResponse),
            x if x == SymbolicLinkAtRequestPart as u16 => Ok(SymbolicLinkAtRequestPart),
            x if x == SymbolicLinkAtResponse as u16 => Ok(SymbolicLinkAtResponse),
            x if x == LinkAtRequestPart as u16 => Ok(LinkAtRequestPart),
            x if x == LinkAtResponse as u16 => Ok(LinkAtResponse),
            x if x == ReadLinkAtRequestPart as u16 => Ok(ReadLinkAtRequestPart),
            x if x == ReadLinkAtResponsePart as u16 => Ok(ReadLinkAtResponsePart),
            x if x == MakeDirectoryAtRequestPart as u16 => Ok(MakeDirectoryAtRequestPart),
            x if x == MakeDirectoryAtResponse as u16 => Ok(MakeDirectoryAtResponse),
            x if x == UpdateFileAccessTimeAtRequestPart as u16 => {
                Ok(UpdateFileAccessTimeAtRequestPart)
            },
            x if x == UpdateFileAccessTimeAtResponse as u16 => Ok(UpdateFileAccessTimeAtResponse),
            x if x == UpdateFileAccessTimeRequest as u16 => Ok(UpdateFileAccessTimeRequest),
            x if x == UpdateFileAccessTimeResponse as u16 => Ok(UpdateFileAccessTimeResponse),
            x if x == FileControlRequest as u16 => Ok(FileControlRequest),
            x if x == FileControlResponse as u16 => Ok(FileControlResponse),
            x if x == CreateSocketRequest as u16 => Ok(CreateSocketRequest),
            x if x == CreateSocketResponse as u16 => Ok(CreateSocketResponse),
            x if x == BindSocketRequest as u16 => Ok(BindSocketRequest),
            x if x == BindSocketResponse as u16 => Ok(BindSocketResponse),
            x if x == ListenSocketRequest as u16 => Ok(ListenSocketRequest),
            x if x == ListenSocketResponse as u16 => Ok(ListenSocketResponse),
            x if x == AcceptSocketRequest as u16 => Ok(AcceptSocketRequest),
            x if x == AcceptSocketResponse as u16 => Ok(AcceptSocketResponse),
            x if x == ShutdownSocketRequest as u16 => Ok(ShutdownSocketRequest),
            x if x == ShutdownSocketResponse as u16 => Ok(ShutdownSocketResponse),
            x if x == ReceiveSocketRequest as u16 => Ok(ReceiveSocketRequest),
            x if x == ReceiveSocketResponse as u16 => Ok(ReceiveSocketResponse),
            x if x == SendSocketRequest as u16 => Ok(SendSocketRequest),
            x if x == SendSocketResponse as u16 => Ok(SendSocketResponse),
            x if x == TimesRequest as u16 => Ok(TimesRequest),
            x if x == TimesResponse as u16 => Ok(TimesResponse),
            x if x == FileChownAtRequestPart as u16 => Ok(FileChownAtRequestPart),
            x if x == FileChownAtResponse as u16 => Ok(FileChownAtResponse),
            x if x == FileChownRequest as u16 => Ok(FileChownRequest),
            x if x == FileChownResponse as u16 => Ok(FileChownResponse),
            x if x == FileChmodAtRequestPart as u16 => Ok(FileChmodAtRequestPart),
            x if x == FileChmodAtResponse as u16 => Ok(FileChmodAtResponse),
            x if x == FileChmodRequest as u16 => Ok(FileChmodRequest),
            x if x == FileChmodResponse as u16 => Ok(FileChmodResponse),
            x if x == ConnectSocketRequest as u16 => Ok(ConnectSocketRequest),
            x if x == ConnectSocketResponse as u16 => Ok(ConnectSocketResponse),
            x if x == CreateSocketPairRequest as u16 => Ok(CreateSocketPairRequest),
            x if x == CreateSocketPairResponse as u16 => Ok(CreateSocketPairResponse),
            x if x == GetPeerNameRequest as u16 => Ok(GetPeerNameRequest),
            x if x == GetPeerNameResponse as u16 => Ok(GetPeerNameResponse),
            x if x == GetSockNameRequest as u16 => Ok(GetSockNameRequest),
            x if x == GetSockNameResponse as u16 => Ok(GetSockNameResponse),
            x if x == PipeRequest as u16 => Ok(PipeRequest),
            x if x == PipeResponse as u16 => Ok(PipeResponse),
            x if x == GetCurrentWorkingDirectoryRequest as u16 => {
                Ok(GetCurrentWorkingDirectoryRequest)
            },
            x if x == GetCurrentWorkingDirectoryResponse as u16 => {
                Ok(GetCurrentWorkingDirectoryResponse)
            },
            x if x == GetCurrentWorkingDirectoryResponsePart as u16 => {
                Ok(GetCurrentWorkingDirectoryResponsePart)
            },
            x if x == GetDirectoryEntriesRequest as u16 => Ok(GetDirectoryEntriesRequest),
            x if x == GetDirectoryEntriesResponse as u16 => Ok(GetDirectoryEntriesResponse),
            x if x == GetDirectoryEntriesResponsePart as u16 => Ok(GetDirectoryEntriesResponsePart),
            x if x == FileChdirRequest as u16 => Ok(FileChdirRequest),
            x if x == FileChdirResponse as u16 => Ok(FileChdirResponse),
            x if x == ChangeDirectoryRequestPart as u16 => Ok(ChangeDirectoryRequestPart),
            x if x == ChangeDirectoryResponse as u16 => Ok(ChangeDirectoryResponse),
            x if x == FileAccessAtRequestPart as u16 => Ok(FileAccessAtRequestPart),
            x if x == FileAccessAtResponse as u16 => Ok(FileAccessAtResponse),
            x if x == GetIdsRequest as u16 => Ok(GetIdsRequest),
            x if x == GetIdsResponse as u16 => Ok(GetIdsResponse),
            x if x == PollRequestPart as u16 => Ok(PollRequestPart),
            x if x == PollResponsePart as u16 => Ok(PollResponsePart),
            x if x == SelectRequest as u16 => Ok(SelectRequest),
            x if x == SelectResponse as u16 => Ok(SelectResponse),
            _ => Err(()),
        }
    }
}

#[repr(C, packed)]
pub struct LinuxDaemonMessage {
    /// Message header.
    pub header: LinuxDaemonMessageHeader,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(LinuxDaemonMessage, Message::PAYLOAD_SIZE);

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
