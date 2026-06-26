// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::{
    fmt,
    mem,
};
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// ResolveFdRequest
//==================================================================================================

///
/// # Description
///
/// Request message of the descriptor-resolution query. libposix sends this to vfsd on a
/// resolution-cache miss to learn the authoritative backend of a flat descriptor number.
///
#[repr(C, packed)]
pub struct ResolveFdRequest {
    /// Process whose descriptor table owns `fd`.
    pub pid: ProcessIdentifier,
    /// The descriptor whose backend route is being queried.
    pub fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ResolveFdRequest, SystemCallMessage::PAYLOAD_SIZE);

impl ResolveFdRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    fn new(pid: ProcessIdentifier, fd: i32) -> Self {
        Self {
            pid,
            fd,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        pid: ProcessIdentifier,
        fd: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: ResolveFdRequest = ResolveFdRequest::new(pid, fd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::ResolveFdRequest, message.into_bytes());
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for ResolveFdRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pid: ProcessIdentifier = self.pid;
        let fd: i32 = self.fd;
        write!(f, "{{ pid: {pid:?}, fd: {fd} }}")
    }
}

//==================================================================================================
// ResolveFdResponse
//==================================================================================================

///
/// # Description
///
/// Response message of the descriptor-resolution query. It carries the descriptor's backend route,
/// the descriptor number that backend expects, and the vfsd table generation the answer was learned
/// at (its coherence epoch).
///
/// The route is carried as a small integer rather than an enum so the wire layout is fixed: `0` is
/// the console (kernel), `1` is vfsd, and `2` is a `networkd` socket. A descriptor with no slot in
/// the process is reported by an error response (non-zero status), not by this message.
///
#[repr(C, packed)]
pub struct ResolveFdResponse {
    /// The backend route: `0` = console, `1` = vfsd, `2` = socket.
    pub route: u32,
    /// The descriptor number the backend expects.
    pub backend_fd: i32,
    /// The vfsd table generation this answer was learned at.
    pub epoch: u64,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(ResolveFdResponse, SystemCallMessage::PAYLOAD_SIZE);

impl ResolveFdResponse {
    /// Wire value for a console-backed descriptor.
    pub const ROUTE_CONSOLE: u32 = 0;
    /// Wire value for a vfsd-served descriptor.
    pub const ROUTE_VFS: u32 = 1;
    /// Wire value for a `networkd` socket descriptor.
    pub const ROUTE_SOCKET: u32 = 2;

    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<u32>()
        - mem::size_of::<i32>()
        - mem::size_of::<u64>();

    fn new(route: u32, backend_fd: i32, epoch: u64) -> Self {
        Self {
            route,
            backend_fd,
            epoch,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        route: u32,
        backend_fd: i32,
        epoch: u64,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: ResolveFdResponse = ResolveFdResponse::new(route, backend_fd, epoch);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::ResolveFdResponse,
            message.into_bytes(),
        );
        Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}
