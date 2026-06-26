// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
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
use sysapi::time::timespec;

//==================================================================================================
// UpdateFileAccessTimeRequest
//==================================================================================================

/// Wire layout: fd (4 bytes) + times[0] (WIRE_SIZE) + times[1] (WIRE_SIZE) + padding.
#[derive(Debug)]
pub struct UpdateFileAccessTimeRequest {
    pub fd: i32,
    pub times: [timespec; 2],
}

// Ensure the fixed-width wire layout fits within the payload, so the manual offsets/slices in
// `from_bytes()`/`into_bytes()` cannot overflow `PAYLOAD_SIZE` and panic at runtime.
::static_assert::assert_eq!(
    UpdateFileAccessTimeRequest::OFFSET_TIMES_1 + timespec::WIRE_SIZE
        <= SystemCallMessage::PAYLOAD_SIZE
);

impl UpdateFileAccessTimeRequest {
    const OFFSET_FD: usize = 0;
    const OFFSET_TIMES_0: usize = mem::size_of::<i32>();
    const OFFSET_TIMES_1: usize = Self::OFFSET_TIMES_0 + timespec::WIRE_SIZE;

    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        let fd = i32::from_ne_bytes(
            bytes[Self::OFFSET_FD..Self::OFFSET_FD + mem::size_of::<i32>()]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid fd field"))?,
        );
        let t0 = timespec::try_from_bytes(
            &bytes[Self::OFFSET_TIMES_0..Self::OFFSET_TIMES_0 + timespec::WIRE_SIZE],
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid times[0] field"))?;
        let t1 = timespec::try_from_bytes(
            &bytes[Self::OFFSET_TIMES_1..Self::OFFSET_TIMES_1 + timespec::WIRE_SIZE],
        )
        .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid times[1] field"))?;
        Ok(Self {
            fd,
            times: [t0, t1],
        })
    }

    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        let mut bytes = [0u8; SystemCallMessage::PAYLOAD_SIZE];
        bytes[Self::OFFSET_FD..Self::OFFSET_FD + mem::size_of::<i32>()]
            .copy_from_slice(&self.fd.to_ne_bytes());
        let t0 = self.times[0].to_bytes();
        bytes[Self::OFFSET_TIMES_0..Self::OFFSET_TIMES_0 + timespec::WIRE_SIZE]
            .copy_from_slice(&t0);
        let t1 = self.times[1].to_bytes();
        bytes[Self::OFFSET_TIMES_1..Self::OFFSET_TIMES_1 + timespec::WIRE_SIZE]
            .copy_from_slice(&t1);
        bytes
    }

    pub fn build(
        tid: ThreadIdentifier,
        fd: i32,
        times: &[timespec; 2],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let request = UpdateFileAccessTimeRequest { fd, times: *times };
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::UpdateFileAccessTimeRequest,
            request.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(ProcessIdentifier::from(i32::from(tid)), tid),
            MessageReceiver::new(destination, ThreadIdentifier::NONE),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}

//==================================================================================================
// UpdateFileAccessTimeResponse
//==================================================================================================

#[repr(C, packed)]
pub struct UpdateFileAccessTimeResponse {
    pub ret: i32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(UpdateFileAccessTimeResponse, SystemCallMessage::PAYLOAD_SIZE);

impl UpdateFileAccessTimeResponse {
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
        let message: UpdateFileAccessTimeResponse = UpdateFileAccessTimeResponse::new(ret);
        let message: SystemCallMessage = SystemCallMessage::new(
            SystemCallMessageHeader::UpdateFileAccessTimeResponse,
            message.into_bytes(),
        );
        let message: Message = Message::new(
            MessageSender::new(source, ThreadIdentifier::NONE),
            MessageReceiver::new(ProcessIdentifier::from(i32::from(tid)), tid),
            message_type,
            None,
            message.into_bytes(),
        );

        message
    }
}
