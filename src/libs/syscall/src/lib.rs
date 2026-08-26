// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "syscall", feature(never_type))] // pthread requires this.
#![cfg_attr(feature = "syscall", feature(c_variadic))] // fcntl requires this.

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

/// Standard library functions.
pub mod stdlib;

/// Client-side path utilities (tilde expansion).
#[cfg(feature = "syscall")]
pub(crate) mod path;

/// Client-side file-descriptor resolution cache.
#[cfg(feature = "syscall")]
pub(crate) mod fdtable;

/// Backend selection for resolved `close()` requests.
#[cfg(feature = "syscall")]
pub(crate) mod close_route;

/// Correlated IPC request helpers.
#[cfg(feature = "syscall")]
pub(crate) mod rpc;

// Safe wrappers.
#[cfg(feature = "syscall")]
pub mod safe;

//==================================================================================================
// Imports
//==================================================================================================

pub use ::config::fds::SOCKET_FD_BASE;
use ::core::{
    convert::TryFrom,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        RequestIdentifier,
        REQUEST_IDENTIFIER_OFFSET,
        REQUEST_IDENTIFIER_PREFIX_SIZE,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Boot-Order Invariants
//==================================================================================================

// Boot-order invariant for the init/root process identifier.
//
// The kernel spawns the guest daemon set (`procd`, `memd`, `vfsd`) on a contiguous pid block
// immediately after the kernel process, then assigns the init/root workload the pid that follows
// the daemon set. The VFS derives the root identity from `ProcessIdentifier::INIT` on that basis,
// so pin the layout with a compile-time check: a daemon renumbering, or a change to the guest
// daemon set, becomes a build error instead of silently re-pointing the root at the wrong process.
//
// This lives in `syscall` because the fixed three-daemon layout pinned below is part of the system
// call interface's boot contract.
mod boot_order_invariants {
    use ::sys::pm::ProcessIdentifier;

    ::static_assert::assert_eq!(ProcessIdentifier::PROCD_RAW == ProcessIdentifier::KERNEL_RAW + 1);
    ::static_assert::assert_eq!(ProcessIdentifier::MEMD_RAW == ProcessIdentifier::PROCD_RAW + 1);
    ::static_assert::assert_eq!(ProcessIdentifier::VFSD_RAW == ProcessIdentifier::MEMD_RAW + 1);
    ::static_assert::assert_eq!(ProcessIdentifier::INIT_RAW == ProcessIdentifier::VFSD_RAW + 1);
    // The init workload follows the *entire* guest daemon set, so the number of guest daemon names
    // must equal the number of guest daemon pids reserved above. Adding a daemon to
    // `GUEST_DAEMON_NAMES` without reserving a pid here (or vice versa) is a compile error.
    ::static_assert::assert_eq!(
        ::config::daemons::GUEST_DAEMON_NAMES.len()
            == [
                ProcessIdentifier::PROCD_RAW,
                ProcessIdentifier::MEMD_RAW,
                ProcessIdentifier::VFSD_RAW,
            ]
            .len()
    );
}

//==================================================================================================
// Structures
//==================================================================================================

/// Identifies the operation carried by a system call message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SystemCallMessageKind {
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
    // Reserved legacy slots. The old 40-byte times response no longer fits in the correlated
    // syscall payload; keep these discriminants so every following wire value remains stable.
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
    HostFsOpenRequest,
    HostFsOpenResponse,
    HostFsCloseRequest,
    HostFsCloseResponse,
    HostFsReadRequest,
    HostFsReadResponse,
    HostFsWriteRequest,
    HostFsWriteResponse,
    HostFsStatRequest,
    HostFsStatResponse,
    HostFsReadDirRequest,
    HostFsReadDirResponse,
    HostFsMkdirRequest,
    HostFsMkdirResponse,
    HostFsRmdirRequest,
    HostFsRmdirResponse,
    HostFsUnlinkRequest,
    HostFsUnlinkResponse,
    HostFsRenameRequest,
    HostFsRenameResponse,
    HostFsLseekRequest,
    HostFsLseekResponse,
    HostFsTruncateRequest,
    HostFsTruncateResponse,
    HostFsFlushRequest,
    HostFsFlushResponse,
    HostFsOpenRequestPart,
    HostFsRenameRequestPart,
    HostFsUnlinkRequestPart,
    HostFsMkdirRequestPart,
    HostFsRmdirRequestPart,
    HostFsSymlinkRequestPart,
    HostFsSymlinkResponse,
    HostFsReadlinkRequest,
    HostFsReadlinkRequestPart,
    HostFsReadlinkResponse,
    HostFsReadlinkResponsePart,
    HostFsLstatRequest,
    HostFsLstatRequestPart,
    HostFsLstatResponse,
    HostFsPathStatRequest,
    HostFsPathStatRequestPart,
    HostFsPathStatResponse,
    HostMountRequestPart,
    HostMountResponse,
    HostUmountRequestPart,
    HostUmountResponse,
    // New variants must be appended here to preserve the on-the-wire discriminant
    // values of existing variants (this enum is `#[repr(u16)]` with implicit,
    // sequential discriminants used directly as IPC/IKC message tags).
    HostFsReadDirResponsePart,
    ResolveFdRequest,
    ResolveFdResponse,
    Dup2Request,
    Dup2Response,
    RegisterSocketRequest,
    RegisterSocketResponse,
    TtyControlRequest,
    TtyControlResponse,
    // Multi-part variant of `SelectRequest`. `select()` is the lone fixed syscall message whose
    // wire layout exceeds the single-message payload once a client-id trailer is reserved, so its
    // request is transported as a `*Part` stream (see `SelectRequest`). Appended here to preserve
    // the on-the-wire discriminants of existing variants.
    SelectRequestPart,
    // Datagram socket I/O carrying an explicit peer address (`sendto()`/`recvfrom()`). Appended
    // here to preserve the on-the-wire discriminants of existing variants.
    SendToSocketRequest,
    SendToSocketResponse,
    ReceiveFromSocketRequest,
    ReceiveFromSocketResponse,
    FileCreationMaskRequest,
    FileCreationMaskResponse,
    PollSocketRequest,
    PollSocketResponse,
    PollInputRequest,
    ConsoleInputAvailable,
    ConsoleInputSubscribe,
    ConsoleReadCancelRequest,
    ConsoleReadCancelResponse,
    PollInputResponse,
    ConsoleReadRetry,
    PipeOpCancelRequest,
    PipeOpCancelResponse,
    PipeReadRetry,
    /// Timestamp continuation for successful hostfs stat operations.
    HostFsStatTimesResponse,
    /// Descriptor-based hostfs ownership request.
    HostFsChownRequest,
    /// Multi-part path-based hostfs ownership request.
    HostFsChownAtRequestPart,
    /// Hostfs ownership response.
    HostFsChownResponse,
    /// Descriptor-based hostfs timestamp update.
    HostFsUpdateTimesRequest,
    HostFsUpdateTimesResponse,
    /// Path-based hostfs timestamp update.
    HostFsUpdateTimesAtRequestPart,
    HostFsUpdateTimesAtResponse,
    /// Multi-part hard-link request for hostfs.
    HostFsLinkRequestPart,
    /// Hard-link response from hostfs.
    HostFsLinkResponse,
    HostFsChmodRequestPart,
    HostFsChmodResponse,
    HostFsFchmodRequest,
    HostFsFchmodResponse,
    HostFsAccessRequestPart,
    HostFsAccessResponse,
}
// Manual TryFrom<u16> implementation for SystemCallMessageKind.
impl TryFrom<u16> for SystemCallMessageKind {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        use SystemCallMessageKind::*;
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
            x if x == HostMountRequestPart as u16 => Ok(HostMountRequestPart),
            x if x == HostMountResponse as u16 => Ok(HostMountResponse),
            x if x == HostUmountRequestPart as u16 => Ok(HostUmountRequestPart),
            x if x == HostUmountResponse as u16 => Ok(HostUmountResponse),
            x if x == HostFsOpenRequest as u16 => Ok(HostFsOpenRequest),
            x if x == HostFsOpenResponse as u16 => Ok(HostFsOpenResponse),
            x if x == HostFsCloseRequest as u16 => Ok(HostFsCloseRequest),
            x if x == HostFsCloseResponse as u16 => Ok(HostFsCloseResponse),
            x if x == HostFsReadRequest as u16 => Ok(HostFsReadRequest),
            x if x == HostFsReadResponse as u16 => Ok(HostFsReadResponse),
            x if x == HostFsWriteRequest as u16 => Ok(HostFsWriteRequest),
            x if x == HostFsWriteResponse as u16 => Ok(HostFsWriteResponse),
            x if x == HostFsStatRequest as u16 => Ok(HostFsStatRequest),
            x if x == HostFsStatResponse as u16 => Ok(HostFsStatResponse),
            x if x == HostFsReadDirRequest as u16 => Ok(HostFsReadDirRequest),
            x if x == HostFsReadDirResponse as u16 => Ok(HostFsReadDirResponse),
            x if x == HostFsReadDirResponsePart as u16 => Ok(HostFsReadDirResponsePart),
            x if x == HostFsMkdirRequest as u16 => Ok(HostFsMkdirRequest),
            x if x == HostFsMkdirResponse as u16 => Ok(HostFsMkdirResponse),
            x if x == HostFsRmdirRequest as u16 => Ok(HostFsRmdirRequest),
            x if x == HostFsRmdirResponse as u16 => Ok(HostFsRmdirResponse),
            x if x == HostFsUnlinkRequest as u16 => Ok(HostFsUnlinkRequest),
            x if x == HostFsUnlinkResponse as u16 => Ok(HostFsUnlinkResponse),
            x if x == HostFsRenameRequest as u16 => Ok(HostFsRenameRequest),
            x if x == HostFsRenameResponse as u16 => Ok(HostFsRenameResponse),
            x if x == HostFsLseekRequest as u16 => Ok(HostFsLseekRequest),
            x if x == HostFsLseekResponse as u16 => Ok(HostFsLseekResponse),
            x if x == HostFsTruncateRequest as u16 => Ok(HostFsTruncateRequest),
            x if x == HostFsTruncateResponse as u16 => Ok(HostFsTruncateResponse),
            x if x == HostFsFlushRequest as u16 => Ok(HostFsFlushRequest),
            x if x == HostFsFlushResponse as u16 => Ok(HostFsFlushResponse),
            x if x == HostFsOpenRequestPart as u16 => Ok(HostFsOpenRequestPart),
            x if x == HostFsRenameRequestPart as u16 => Ok(HostFsRenameRequestPart),
            x if x == HostFsUnlinkRequestPart as u16 => Ok(HostFsUnlinkRequestPart),
            x if x == HostFsMkdirRequestPart as u16 => Ok(HostFsMkdirRequestPart),
            x if x == HostFsRmdirRequestPart as u16 => Ok(HostFsRmdirRequestPart),
            x if x == HostFsSymlinkRequestPart as u16 => Ok(HostFsSymlinkRequestPart),
            x if x == HostFsSymlinkResponse as u16 => Ok(HostFsSymlinkResponse),
            x if x == HostFsReadlinkRequest as u16 => Ok(HostFsReadlinkRequest),
            x if x == HostFsReadlinkRequestPart as u16 => Ok(HostFsReadlinkRequestPart),
            x if x == HostFsReadlinkResponse as u16 => Ok(HostFsReadlinkResponse),
            x if x == HostFsReadlinkResponsePart as u16 => Ok(HostFsReadlinkResponsePart),
            x if x == HostFsLstatRequest as u16 => Ok(HostFsLstatRequest),
            x if x == HostFsLstatRequestPart as u16 => Ok(HostFsLstatRequestPart),
            x if x == HostFsLstatResponse as u16 => Ok(HostFsLstatResponse),
            x if x == HostFsPathStatRequest as u16 => Ok(HostFsPathStatRequest),
            x if x == HostFsPathStatRequestPart as u16 => Ok(HostFsPathStatRequestPart),
            x if x == HostFsPathStatResponse as u16 => Ok(HostFsPathStatResponse),
            x if x == ResolveFdRequest as u16 => Ok(ResolveFdRequest),
            x if x == ResolveFdResponse as u16 => Ok(ResolveFdResponse),
            x if x == Dup2Request as u16 => Ok(Dup2Request),
            x if x == Dup2Response as u16 => Ok(Dup2Response),
            x if x == RegisterSocketRequest as u16 => Ok(RegisterSocketRequest),
            x if x == RegisterSocketResponse as u16 => Ok(RegisterSocketResponse),
            x if x == TtyControlRequest as u16 => Ok(TtyControlRequest),
            x if x == TtyControlResponse as u16 => Ok(TtyControlResponse),
            x if x == SelectRequestPart as u16 => Ok(SelectRequestPart),
            x if x == SendToSocketRequest as u16 => Ok(SendToSocketRequest),
            x if x == SendToSocketResponse as u16 => Ok(SendToSocketResponse),
            x if x == ReceiveFromSocketRequest as u16 => Ok(ReceiveFromSocketRequest),
            x if x == ReceiveFromSocketResponse as u16 => Ok(ReceiveFromSocketResponse),
            x if x == FileCreationMaskRequest as u16 => Ok(FileCreationMaskRequest),
            x if x == FileCreationMaskResponse as u16 => Ok(FileCreationMaskResponse),
            x if x == PollSocketRequest as u16 => Ok(PollSocketRequest),
            x if x == PollSocketResponse as u16 => Ok(PollSocketResponse),
            x if x == PollInputRequest as u16 => Ok(PollInputRequest),
            x if x == ConsoleInputAvailable as u16 => Ok(ConsoleInputAvailable),
            x if x == ConsoleInputSubscribe as u16 => Ok(ConsoleInputSubscribe),
            x if x == ConsoleReadCancelRequest as u16 => Ok(ConsoleReadCancelRequest),
            x if x == ConsoleReadCancelResponse as u16 => Ok(ConsoleReadCancelResponse),
            x if x == PollInputResponse as u16 => Ok(PollInputResponse),
            x if x == ConsoleReadRetry as u16 => Ok(ConsoleReadRetry),
            x if x == PipeOpCancelRequest as u16 => Ok(PipeOpCancelRequest),
            x if x == PipeOpCancelResponse as u16 => Ok(PipeOpCancelResponse),
            x if x == PipeReadRetry as u16 => Ok(PipeReadRetry),
            x if x == HostFsStatTimesResponse as u16 => Ok(HostFsStatTimesResponse),
            x if x == HostFsChownRequest as u16 => Ok(HostFsChownRequest),
            x if x == HostFsChownAtRequestPart as u16 => Ok(HostFsChownAtRequestPart),
            x if x == HostFsChownResponse as u16 => Ok(HostFsChownResponse),
            x if x == HostFsUpdateTimesRequest as u16 => Ok(HostFsUpdateTimesRequest),
            x if x == HostFsUpdateTimesResponse as u16 => Ok(HostFsUpdateTimesResponse),
            x if x == HostFsUpdateTimesAtRequestPart as u16 => Ok(HostFsUpdateTimesAtRequestPart),
            x if x == HostFsUpdateTimesAtResponse as u16 => Ok(HostFsUpdateTimesAtResponse),
            x if x == HostFsLinkRequestPart as u16 => Ok(HostFsLinkRequestPart),
            x if x == HostFsLinkResponse as u16 => Ok(HostFsLinkResponse),
            x if x == HostFsChmodRequestPart as u16 => Ok(HostFsChmodRequestPart),
            x if x == HostFsChmodResponse as u16 => Ok(HostFsChmodResponse),
            x if x == HostFsFchmodRequest as u16 => Ok(HostFsFchmodRequest),
            x if x == HostFsFchmodResponse as u16 => Ok(HostFsFchmodResponse),
            x if x == HostFsAccessRequestPart as u16 => Ok(HostFsAccessRequestPart),
            x if x == HostFsAccessResponse as u16 => Ok(HostFsAccessResponse),
            _ => Err(()),
        }
    }
}

impl SystemCallMessageKind {
    /// Returns `true` if this header identifies a host filesystem operation.
    pub fn is_hostfs(&self) -> bool {
        matches!(
            self,
            Self::HostFsOpenRequest
                | Self::HostFsOpenResponse
                | Self::HostFsUpdateTimesRequest
                | Self::HostFsUpdateTimesResponse
                | Self::HostFsUpdateTimesAtRequestPart
                | Self::HostFsUpdateTimesAtResponse
                | Self::HostFsCloseRequest
                | Self::HostFsCloseResponse
                | Self::HostFsReadRequest
                | Self::HostFsReadResponse
                | Self::HostFsWriteRequest
                | Self::HostFsWriteResponse
                | Self::HostFsStatRequest
                | Self::HostFsStatResponse
                | Self::HostFsReadDirRequest
                | Self::HostFsReadDirResponse
                | Self::HostFsReadDirResponsePart
                | Self::HostFsMkdirRequest
                | Self::HostFsMkdirResponse
                | Self::HostFsRmdirRequest
                | Self::HostFsRmdirResponse
                | Self::HostFsUnlinkRequest
                | Self::HostFsUnlinkResponse
                | Self::HostFsRenameRequest
                | Self::HostFsRenameResponse
                | Self::HostFsLseekRequest
                | Self::HostFsLseekResponse
                | Self::HostFsTruncateRequest
                | Self::HostFsTruncateResponse
                | Self::HostFsFlushRequest
                | Self::HostFsFlushResponse
                | Self::HostFsOpenRequestPart
                | Self::HostFsRenameRequestPart
                | Self::HostFsUnlinkRequestPart
                | Self::HostFsMkdirRequestPart
                | Self::HostFsRmdirRequestPart
                | Self::HostFsSymlinkRequestPart
                | Self::HostFsSymlinkResponse
                | Self::HostFsReadlinkRequest
                | Self::HostFsReadlinkRequestPart
                | Self::HostFsReadlinkResponse
                | Self::HostFsReadlinkResponsePart
                | Self::HostFsLstatRequest
                | Self::HostFsLstatRequestPart
                | Self::HostFsLstatResponse
                | Self::HostFsPathStatRequest
                | Self::HostFsPathStatRequestPart
                | Self::HostFsPathStatResponse
                | Self::HostFsStatTimesResponse
                | Self::HostFsChownRequest
                | Self::HostFsChownAtRequestPart
                | Self::HostFsChownResponse
                | Self::HostFsLinkRequestPart
                | Self::HostFsLinkResponse
                | Self::HostFsChmodRequestPart
                | Self::HostFsChmodResponse
                | Self::HostFsFchmodRequest
                | Self::HostFsFchmodResponse
                | Self::HostFsAccessRequestPart
                | Self::HostFsAccessResponse
        )
    }

    /// Returns the corresponding response kind for a hostfs request kind.
    ///
    /// This provides an explicit mapping instead of relying on enum discriminant arithmetic.
    /// Returns `None` if this is not a hostfs request kind.
    pub fn hostfs_response_kind(&self) -> Option<Self> {
        match self {
            Self::HostFsOpenRequest | Self::HostFsOpenRequestPart => Some(Self::HostFsOpenResponse),
            Self::HostFsCloseRequest => Some(Self::HostFsCloseResponse),
            Self::HostFsReadRequest => Some(Self::HostFsReadResponse),
            Self::HostFsWriteRequest => Some(Self::HostFsWriteResponse),
            Self::HostFsStatRequest => Some(Self::HostFsStatResponse),
            Self::HostFsReadDirRequest => Some(Self::HostFsReadDirResponse),
            Self::HostFsMkdirRequest | Self::HostFsMkdirRequestPart => {
                Some(Self::HostFsMkdirResponse)
            },
            Self::HostFsRmdirRequest | Self::HostFsRmdirRequestPart => {
                Some(Self::HostFsRmdirResponse)
            },
            Self::HostFsUnlinkRequest | Self::HostFsUnlinkRequestPart => {
                Some(Self::HostFsUnlinkResponse)
            },
            Self::HostFsRenameRequest | Self::HostFsRenameRequestPart => {
                Some(Self::HostFsRenameResponse)
            },
            Self::HostFsLseekRequest => Some(Self::HostFsLseekResponse),
            Self::HostFsTruncateRequest => Some(Self::HostFsTruncateResponse),
            Self::HostFsFlushRequest => Some(Self::HostFsFlushResponse),
            Self::HostFsSymlinkRequestPart => Some(Self::HostFsSymlinkResponse),
            Self::HostFsReadlinkRequest | Self::HostFsReadlinkRequestPart => {
                Some(Self::HostFsReadlinkResponse)
            },
            Self::HostFsLstatRequest | Self::HostFsLstatRequestPart => {
                Some(Self::HostFsLstatResponse)
            },
            Self::HostFsPathStatRequest | Self::HostFsPathStatRequestPart => {
                Some(Self::HostFsPathStatResponse)
            },
            Self::HostFsChownRequest | Self::HostFsChownAtRequestPart => {
                Some(Self::HostFsChownResponse)
            },
            Self::HostFsUpdateTimesRequest => Some(Self::HostFsUpdateTimesResponse),
            Self::HostFsUpdateTimesAtRequestPart => Some(Self::HostFsUpdateTimesAtResponse),
            Self::HostFsLinkRequestPart => Some(Self::HostFsLinkResponse),
            Self::HostFsChmodRequestPart => Some(Self::HostFsChmodResponse),
            Self::HostFsFchmodRequest => Some(Self::HostFsFchmodResponse),
            Self::HostFsAccessRequestPart => Some(Self::HostFsAccessResponse),
            _ => None,
        }
    }

    /// Returns `true` if this header is a hostfs response variant.
    ///
    /// `HostFsReadlinkResponsePart` and `HostFsReadDirResponsePart` are intentionally excluded
    /// because vfsd must assemble their bodies before dispatch. Their request-ID field echoes the
    /// hostfs operation identifier, which also lives in the first four bytes of the assembled body.
    /// They are dispatched through `is_hostfs_multipart_response` and the dedicated assemblers in
    /// vfsd.
    pub fn is_hostfs_response(&self) -> bool {
        matches!(
            self,
            Self::HostFsOpenResponse
                | Self::HostFsCloseResponse
                | Self::HostFsReadResponse
                | Self::HostFsWriteResponse
                | Self::HostFsStatResponse
                | Self::HostFsReadDirResponse
                | Self::HostFsMkdirResponse
                | Self::HostFsRmdirResponse
                | Self::HostFsUnlinkResponse
                | Self::HostFsRenameResponse
                | Self::HostFsLseekResponse
                | Self::HostFsTruncateResponse
                | Self::HostFsFlushResponse
                | Self::HostFsSymlinkResponse
                | Self::HostFsReadlinkResponse
                | Self::HostFsLstatResponse
                | Self::HostFsPathStatResponse
                | Self::HostFsStatTimesResponse
                | Self::HostFsChownResponse
                | Self::HostFsUpdateTimesResponse
                | Self::HostFsUpdateTimesAtResponse
                | Self::HostFsLinkResponse
                | Self::HostFsChmodResponse
                | Self::HostFsFchmodResponse
                | Self::HostFsAccessResponse
        )
    }

    /// Returns `true` if this header represents a *part* of a multi-part hostfs *response* stream.
    pub fn is_hostfs_multipart_response(&self) -> bool {
        matches!(self, Self::HostFsReadlinkResponsePart | Self::HostFsReadDirResponsePart)
    }
}

/// Header shared by correlated system call requests and responses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C, packed)]
pub struct SystemCallMessageHeader {
    /// Operation carried by the message.
    kind: SystemCallMessageKind,
    /// Identifier that correlates a response with its request.
    request_id: RequestIdentifier,
}
::static_assert::assert_eq_size!(SystemCallMessageHeader, REQUEST_IDENTIFIER_PREFIX_SIZE);
::static_assert::assert_eq!(
    mem::offset_of!(SystemCallMessageHeader, request_id) == REQUEST_IDENTIFIER_OFFSET
);

#[repr(C, packed)]
/// Correlated system call message carried in an IPC payload.
pub struct SystemCallMessage {
    /// Message header.
    header: SystemCallMessageHeader,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}
::static_assert::assert_eq_size!(SystemCallMessage, Message::PAYLOAD_SIZE);
::static_assert::assert_eq!(
    mem::offset_of!(SystemCallMessage, payload) == REQUEST_IDENTIFIER_PREFIX_SIZE
);

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Process identifier used to route system calls to the host I/O handler.
///
pub const HOST_IO: ProcessIdentifier = ProcessIdentifier::KERNEL;

///
/// # Description
///
/// Process identifier of the network daemon.
///
pub const NETWORKD: ProcessIdentifier = ProcessIdentifier::NETWORKD;

/// Destination process for networking system call requests.
pub const NETWORK_DESTINATION: ProcessIdentifier = NETWORKD;

/// Source process for networking system call responses.
///
/// Must match the daemon that handles the request so responses carry the correct origin.
pub const NETWORK_SOURCE: ProcessIdentifier = NETWORKD;

///
/// # Description
///
/// Process identifier of the VFS daemon.
///
pub const VFSD: ProcessIdentifier = ProcessIdentifier::VFSD;

/// Destination process for VFS/filesystem system call requests.
///
/// Requests are routed to the guest-side vfsd daemon.
pub const VFS_DESTINATION: ProcessIdentifier = VFSD;

/// Message type for VFS/filesystem system call requests.
///
/// Messages use local IPC routed within the guest kernel.
pub const VFS_MESSAGE_TYPE: ::sys::ipc::MessageType = ::sys::ipc::MessageType::Ipc;

/// Process identifier for push/pull data transfers in VFS operations.
///
/// Data is transferred directly between the caller and vfsd via rendezvous.
pub const VFS_PUSH_PULL_PID: ProcessIdentifier = ProcessIdentifier::VFSD;

/// Thread identifier for push/pull data transfers in VFS operations.
pub const VFS_PUSH_PULL_TID: ::sys::pm::ThreadIdentifier = ::sys::pm::ThreadIdentifier::VFSD;

//==================================================================================================
// Implementations
//==================================================================================================

impl SystemCallMessageHeader {
    const fn new(kind: SystemCallMessageKind, request_id: RequestIdentifier) -> Self {
        Self { kind, request_id }
    }

    /// Returns the operation carried by the message.
    pub const fn kind(self) -> SystemCallMessageKind {
        self.kind
    }

    /// Returns the identifier that correlates a response with its request.
    pub const fn request_id(self) -> RequestIdentifier {
        self.request_id
    }
}

impl SystemCallMessage {
    pub const PAYLOAD_SIZE: usize = Message::PAYLOAD_SIZE - REQUEST_IDENTIFIER_PREFIX_SIZE;

    pub fn new(kind: SystemCallMessageKind, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self {
            header: SystemCallMessageHeader::new(kind, RequestIdentifier::NONE),
            payload,
        }
    }

    /// Returns the message header.
    pub const fn header(&self) -> SystemCallMessageHeader {
        self.header
    }

    /// Returns the operation carried by the message.
    pub const fn kind(&self) -> SystemCallMessageKind {
        self.header.kind()
    }

    /// Returns the identifier that correlates a response with its request.
    pub const fn request_id(&self) -> RequestIdentifier {
        self.header.request_id()
    }

    pub fn try_from_bytes(bytes: [u8; Message::PAYLOAD_SIZE]) -> Result<Self, Error> {
        // Check if message header is valid.
        let _kind: SystemCallMessageKind =
            SystemCallMessageKind::try_from(u16::from_ne_bytes([bytes[0], bytes[1]]))
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid message header"))?;

        let message: SystemCallMessage = unsafe { mem::transmute(bytes) };

        Ok(message)
    }

    pub fn into_bytes(self) -> [u8; Message::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

// A nested request may have to set aside complete multipart responses for every other active
// request. `getdents()` has the largest legal response in the syscall protocol, so this assertion
// keeps the shared stash bound synchronized with protocol growth.
::static_assert::assert_eq!(
    ::sys::ipc::RESPONSE_STASH_CAPACITY
        >= (::sys::ipc::MAX_ACTIVE_REQUESTS - 1)
            * crate::dirent::message::GetDirectoryEntriesResponse::MAX_SIZE
                .div_ceil(crate::message::SystemCallMessagePart::PAYLOAD_SIZE)
);

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_call_message_request_id_round_trip() {
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(0x1234_5678);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageKind::CloseRequest,
            [0; SystemCallMessage::PAYLOAD_SIZE],
        );
        let bytes: [u8; Message::PAYLOAD_SIZE] = message.into_bytes();
        let mut outer: Message = Message::new(
            ::sys::ipc::MessageSender::KERNEL,
            ::sys::ipc::MessageReceiver::KERNEL,
            ::sys::ipc::MessageType::Ipc,
            None,
            bytes,
        );
        request_id.write_to(&mut outer);
        assert_eq!(
            &outer.payload
                [REQUEST_IDENTIFIER_OFFSET..REQUEST_IDENTIFIER_OFFSET + RequestIdentifier::SIZE],
            &request_id.raw().to_ne_bytes()
        );
        assert_eq!(RequestIdentifier::read_from(&outer), request_id);

        let decoded: SystemCallMessage =
            SystemCallMessage::try_from_bytes(outer.payload).expect("message should decode");
        assert_eq!(decoded.kind(), SystemCallMessageKind::CloseRequest);
        assert_eq!(decoded.request_id(), request_id);
    }
}
