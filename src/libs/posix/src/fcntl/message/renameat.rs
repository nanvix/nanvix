// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    limits,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::{
    fmt,
    mem,
};
use ::nvx::{
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// RenameAtRequest
//==================================================================================================

#[repr(C, packed)]
pub struct RenameAtRequest {
    pub olddirfd: i32,
    pub oldpath: [u8; limits::NAME_MAX],
    pub newdirfd: i32,
    pub newpath: [u8; limits::NAME_MAX],
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(RenameAtRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl RenameAtRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - limits::NAME_MAX
        - mem::size_of::<i32>()
        - limits::NAME_MAX;

    fn new(
        olddirfd: i32,
        oldpath: [u8; limits::NAME_MAX],
        newdirfd: i32,
        newpath: [u8; limits::NAME_MAX],
    ) -> Self {
        Self {
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
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
        pid: ProcessIdentifier,
        olddirfd: i32,
        oldpath: &str,
        newdirfd: i32,
        newpath: &str,
    ) -> Result<Message, Error> {
        // Check if oldpath is too long.
        if oldpath.len() >= limits::NAME_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "oldpath is too long"));
        }

        // Check if newpath is too long.
        if newpath.len() >= limits::NAME_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "newpath is too long"));
        }

        let mut oldpath_bytes: [u8; limits::NAME_MAX] = [0u8; limits::NAME_MAX];
        oldpath_bytes[..oldpath.len()].copy_from_slice(oldpath.as_bytes());

        let mut newpath_bytes: [u8; limits::NAME_MAX] = [0u8; limits::NAME_MAX];
        newpath_bytes[..newpath.len()].copy_from_slice(newpath.as_bytes());

        let message: RenameAtRequest =
            RenameAtRequest::new(olddirfd, oldpath_bytes, newdirfd, newpath_bytes);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::RenameAtRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        Ok(message)
    }
}

impl fmt::Debug for RenameAtRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let olddirfd: i32 = self.olddirfd;
        let newdirfd: i32 = self.newdirfd;

        write!(
            f,
            "{{ olddirfd: {}, oldpath: {:?}, newdirfd: {}, newpath: {:?} }}",
            olddirfd,
            unsafe { core::str::from_utf8_unchecked(&self.oldpath) },
            newdirfd,
            unsafe { core::str::from_utf8_unchecked(&self.newpath) }
        )
    }
}

//==================================================================================================
// RenameAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct RenameAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(RenameAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl RenameAtResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    fn new(ret: i32) -> Self {
        Self {
            ret,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    pub fn from_bytes(bytes: [u8; LinuxDaemonMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    fn into_bytes(self) -> [u8; LinuxDaemonMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    pub fn build(pid: ProcessIdentifier, ret: i32) -> Message {
        let message: RenameAtResponse = RenameAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::RenameAtResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
