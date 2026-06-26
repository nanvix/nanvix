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
// RegisterSocketRequest
//==================================================================================================

///
/// # Description
///
/// Request message that asks vfsd to allocate a flat descriptor slot for a socket endpoint that
/// `networkd` has already created. libposix sends this as the second step of socket creation: once
/// `networkd` returns the endpoint's remote descriptor, vfsd hands out the lowest free flat number
/// and binds it to a socket routing token holding `remote_fd`.
///
#[repr(C, packed)]
pub struct RegisterSocketRequest {
    /// The descriptor `networkd` assigned to the endpoint (its remote fd).
    pub remote_fd: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(RegisterSocketRequest, SystemCallMessage::PAYLOAD_SIZE);

impl RegisterSocketRequest {
    pub const PADDING_SIZE: usize = SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(remote_fd: i32) -> Self {
        Self {
            remote_fd,
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
        remote_fd: i32,
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: RegisterSocketRequest = RegisterSocketRequest::new(remote_fd);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::RegisterSocketRequest,
            message.into_bytes(),
        );
        Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        )
    }
}

impl fmt::Debug for RegisterSocketRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let remote_fd: i32 = self.remote_fd;
        write!(f, "{{ remote_fd: {remote_fd} }}")
    }
}

//==================================================================================================
// RegisterSocketResponse
//==================================================================================================

///
/// # Description
///
/// Response message of [`RegisterSocketRequest`]. It carries the flat descriptor vfsd allocated for
/// the socket and the vfsd table generation the slot was created at (its coherence epoch), so the
/// caller can seed its resolution cache with an entry that is coherent against the table state that
/// produced it.
///
#[repr(C, packed)]
pub struct RegisterSocketResponse {
    /// The flat descriptor vfsd allocated for the socket.
    pub fd: i32,
    /// The vfsd table generation this slot was created at.
    pub epoch: u64,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(RegisterSocketResponse, SystemCallMessage::PAYLOAD_SIZE);

impl RegisterSocketResponse {
    pub const PADDING_SIZE: usize =
        SystemCallMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<u64>();

    fn new(fd: i32, epoch: u64) -> Self {
        Self {
            fd,
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
        fd: i32,
        epoch: u64,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: RegisterSocketResponse = RegisterSocketResponse::new(fd, epoch);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::RegisterSocketResponse,
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

impl fmt::Debug for RegisterSocketResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fd: i32 = self.fd;
        let epoch: u64 = self.epoch;
        write!(f, "{{ fd: {fd}, epoch: {epoch} }}")
    }
}
