// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    limits,
    sys::types::mode_t,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::core::{
    ffi,
    fmt::Debug,
    mem,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// OpenAtRequest
//==================================================================================================

#[repr(C, packed)]
pub struct OpenAtRequest {
    pub dirfd: i32,
    pub flags: ffi::c_int,
    pub mode: mode_t,
    pub pathname: [u8; limits::NAME_MAX],
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(OpenAtRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl OpenAtRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - mem::size_of::<ffi::c_int>()
        - mem::size_of::<mode_t>()
        - limits::NAME_MAX;

    fn new(dirfd: i32, pathname: [u8; limits::NAME_MAX], flags: ffi::c_int, mode: mode_t) -> Self {
        Self {
            dirfd,
            flags,
            mode,
            pathname,
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
        dirfd: i32,
        pathname: &str,
        flags: ffi::c_int,
        mode: mode_t,
    ) -> Result<Message, Error> {
        // Check if pathname is not too long.
        if pathname.len() > limits::NAME_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "pathname too long"));
        }

        let mut pathname_bytes: [u8; limits::NAME_MAX] = [0u8; limits::NAME_MAX];
        pathname_bytes[..pathname.len()].copy_from_slice(pathname.as_bytes());

        let message: OpenAtRequest = OpenAtRequest::new(dirfd, pathname_bytes, flags, mode);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::OpenAtRequest, message.into_bytes());
        let message: Message = Message::new(
            pid,
            crate::LINUXD,
            nvx::ipc::MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        Ok(message)
    }
}

impl Debug for OpenAtRequest {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let dirfd: i32 = self.dirfd;
        let flags: ffi::c_int = self.flags;
        let mode: mode_t = self.mode;
        write!(f, "{{ dirfd: {:?}, flags: {:?}, mode: {:?} }}", dirfd, flags, mode)
    }
}

//==================================================================================================
// OpenAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct OpenAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(OpenAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl OpenAtResponse {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE - mem::size_of::<i32>();

    pub fn new(ret: i32) -> Self {
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
        let message: OpenAtResponse = OpenAtResponse::new(ret);
        let message: LinuxDaemonMessage =
            LinuxDaemonMessage::new(LinuxDaemonMessageHeader::OpenAtResponse, message.into_bytes());
        let message: Message = Message::new(
            crate::LINUXD,
            pid,
            nvx::ipc::MessageType::Ikc,
            None,
            message.into_bytes(),
        );

        message
    }
}
