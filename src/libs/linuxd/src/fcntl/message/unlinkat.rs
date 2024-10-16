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
    ffi,
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
// UnlinkAtRequest
//==================================================================================================

#[repr(C, packed)]
pub struct UnlinkAtRequest {
    pub dirfd: i32,
    pub pathname: [u8; limits::NAME_MAX],
    pub flags: ffi::c_int,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(UnlinkAtRequest, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UnlinkAtRequest {
    pub const PADDING_SIZE: usize = LinuxDaemonMessage::PAYLOAD_SIZE
        - mem::size_of::<i32>()
        - limits::NAME_MAX
        - mem::size_of::<ffi::c_int>();

    fn new(dirfd: i32, pathname: [u8; limits::NAME_MAX], flags: ffi::c_int) -> Self {
        Self {
            dirfd,
            pathname,
            flags,
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
    ) -> Result<Message, Error> {
        // Check if pathname is too long.
        if pathname.len() >= limits::NAME_MAX {
            return Err(Error::new(ErrorCode::InvalidArgument, "pathname is too long"));
        }

        let mut pathname_bytes: [u8; limits::NAME_MAX] = [0u8; limits::NAME_MAX];
        pathname_bytes[..pathname.len()].copy_from_slice(pathname.as_bytes());

        let message: UnlinkAtRequest = UnlinkAtRequest::new(dirfd, pathname_bytes, flags);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UnlinkAtRequest,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(pid, crate::LINUXD, MessageType::Ikc, None, message.into_bytes());

        Ok(message)
    }
}

impl fmt::Debug for UnlinkAtRequest {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let dirfd: i32 = self.dirfd;
        let pathname: &str = unsafe { core::str::from_utf8_unchecked(&self.pathname) };
        let flags: ffi::c_int = self.flags;
        write!(f, "{{ dirfd: {}, pathname: {}, flags: {} }}", dirfd, pathname, flags)
    }
}

//==================================================================================================
// UnlinkAtResponse
//==================================================================================================

#[repr(C, packed)]
pub struct UnlinkAtResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::nvx::sys::static_assert_size!(UnlinkAtResponse, LinuxDaemonMessage::PAYLOAD_SIZE);

impl UnlinkAtResponse {
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
        let message: UnlinkAtResponse = UnlinkAtResponse::new(ret);
        let message: LinuxDaemonMessage = LinuxDaemonMessage::new(
            LinuxDaemonMessageHeader::UnlinkAtResponse,
            message.into_bytes(),
        );
        let message: Message =
            Message::new(crate::LINUXD, pid, MessageType::Ikc, None, message.into_bytes());

        message
    }
}
