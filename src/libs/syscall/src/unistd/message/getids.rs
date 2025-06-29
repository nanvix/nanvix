// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::mem;
use ::sys::{
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
    },
    pm::ThreadIdentifier,
};
use sysapi::sys_types::{
    gid_t,
    uid_t,
};

//==================================================================================================
// GetIdsRequest
//==================================================================================================

#[repr(C, packed)]
pub struct GetIdsRequest {
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(GetIdsRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetIdsRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE;

    fn new() -> Self {
        Self {
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(tid: ThreadIdentifier) -> Message {
        let message: GetIdsRequest = GetIdsRequest::new();
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::GetIdsRequest, message.into_bytes());
        Message::new(
            MessageSender::from(tid),
            MessageReceiver::from(crate::LINUXD),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}

//==================================================================================================
// GetIdsResponse
//==================================================================================================

#[repr(C, packed)]
pub struct GetIdsResponse {
    pub uid: uid_t,
    pub gid: gid_t,
    pub euid: uid_t,
    pub egid: gid_t,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(GetIdsResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl GetIdsResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<uid_t>() * 2 // Size of `uid` + `euid`
        - mem::size_of::<gid_t>() * 2; // Size of `gid` + `egid`

    fn new(uid: u32, gid: u32, euid: u32, egid: u32) -> Self {
        Self {
            uid,
            gid,
            euid,
            egid,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(
        tid: ThreadIdentifier,
        uid: uid_t,
        gid: gid_t,
        euid: uid_t,
        egid: gid_t,
    ) -> Message {
        let message: GetIdsResponse = GetIdsResponse::new(uid, gid, euid, egid);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::GetIdsResponse, message.into_bytes());
        Message::new(
            MessageSender::from(crate::LINUXD),
            MessageReceiver::from(tid),
            MessageType::Ikc,
            None,
            message.into_bytes(),
        )
    }
}
