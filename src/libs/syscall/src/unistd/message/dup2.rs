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
// Dup2Request
//==================================================================================================

///
/// # Description
///
/// Request message of the `dup2()` system call. It carries the source and target descriptor numbers
/// as the flat slot numbers vfsd owns, so vfsd re-points `newfd` at `oldfd`'s open file description
/// directly on its authoritative table.
///
#[repr(C, packed)]
pub struct Dup2Request {
    /// Descriptor whose open file description is duplicated.
    pub oldfd: i32,
    /// Descriptor that is made to alias `oldfd`, closing whatever it previously referred to.
    pub newfd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(Dup2Request, SystemCallMessage::PAYLOAD_SIZE);

impl Dup2Request {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<i32>();

    fn new(oldfd: i32, newfd: i32) -> Self {
        Self {
            oldfd,
            newfd,
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
        oldfd: i32,
        newfd: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: Dup2Request = Dup2Request::new(oldfd, newfd);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::Dup2Request, message.into_bytes());
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for Dup2Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let oldfd: i32 = self.oldfd;
        let newfd: i32 = self.newfd;
        write!(f, "{{ oldfd: {oldfd}, newfd: {newfd} }}")
    }
}

//==================================================================================================
// Dup2Response
//==================================================================================================

///
/// # Description
///
/// Response message of the `dup2()` system call. It carries the descriptor that now aliases the
/// source, which equals `newfd` on success.
///
#[repr(C, packed)]
pub struct Dup2Response {
    /// The descriptor that now aliases the source (equals `newfd` on success).
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(Dup2Response, SystemCallMessage::PAYLOAD_SIZE);

impl Dup2Response {
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
        let message: Dup2Response = Dup2Response::new(ret);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::Dup2Response, message.into_bytes());
        Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for Dup2Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ret: i32 = self.ret;
        write!(f, "{{ ret: {ret} }}")
    }
}
