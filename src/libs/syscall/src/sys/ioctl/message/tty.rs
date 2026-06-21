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
// TtyControlRequest
//==================================================================================================

///
/// # Description
///
/// Request message of a terminal-control `ioctl` (`TCGETS`/`TCSETS`/`TIOCGWINSZ`/`TIOCSWINSZ`).
///
/// The message carries only the metadata; the `termios`/`winsize` payload is transferred out of band
/// via a push/pull bulk transfer (the payload does not fit in a single IPC message). For a *get*
/// request vfsd pushes the payload to the caller; for a *set* request the caller pushes the payload
/// to vfsd.
///
#[repr(C, packed)]
pub struct TtyControlRequest {
    /// Flat descriptor whose terminal is addressed (vfsd resolves it against the caller's table).
    pub fd: i32,
    /// The terminal-control request code.
    pub request: i32,
    /// Length, in bytes, of the out-of-band payload.
    pub len: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TtyControlRequest, SystemCallMessage::PAYLOAD_SIZE);

impl TtyControlRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<i32>()
        - mem::size_of::<u32>();

    fn new(fd: i32, request: i32, len: u32) -> Self {
        Self {
            fd,
            request,
            len,
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
        fd: i32,
        request: i32,
        len: u32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: TtyControlRequest = TtyControlRequest::new(fd, request, len);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::TtyControlRequest,
            message.into_bytes(),
        );
        Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(destination),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for TtyControlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fd: i32 = self.fd;
        let request: i32 = self.request;
        let len: u32 = self.len;
        write!(f, "{{ fd: {fd}, request: {request:#x}, len: {len} }}")
    }
}

//==================================================================================================
// TtyControlResponse
//==================================================================================================

///
/// # Description
///
/// Response message of a terminal-control `ioctl`. Success is signaled by a zero status on the
/// carrying [`Message`]; the embedded `ret` is the value the `ioctl` returns to the caller (`0`).
///
#[repr(C, packed)]
pub struct TtyControlResponse {
    /// The value the `ioctl` returns on success.
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(TtyControlResponse, SystemCallMessage::PAYLOAD_SIZE);

impl TtyControlResponse {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
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
        ret: i32,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: TtyControlResponse = TtyControlResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::TtyControlResponse,
            message.into_bytes(),
        );
        Message::new(
            MessageSender::from(source),
            MessageReceiver::from(tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for TtyControlResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ret: i32 = self.ret;
        write!(f, "{{ ret: {ret} }}")
    }
}
