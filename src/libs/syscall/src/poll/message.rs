// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        LinuxDaemonMessagePart,
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
    },
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
use ::core::mem;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use ::sysapi::limits::OPEN_MAX;

//==================================================================================================
// Constants
//==================================================================================================

// Ensure that the maximum number of file descriptors can be encoded in a `u8`.
::static_assert::assert_eq!(OPEN_MAX < u8::MAX as usize);

//==================================================================================================
// Structure
//==================================================================================================

#[derive(Debug)]
pub struct PollRequest {
    /// Number of file descriptors to poll.
    pub nfds: u8,
    /// Timeout for the poll operation, in milliseconds.
    pub timeout: i32,
    /// File descriptors to poll (length == nfds).
    pub fds: Vec<i32>,
    /// Events to poll for on each file descriptor (length == nfds).
    pub events: Vec<i16>,
}

impl PollRequest {
    /// Size of `nfds` field.
    pub const SIZE_OF_NFDS: usize = mem::size_of::<u8>();
    /// Size of `timeout` field.
    pub const SIZE_OF_TIMEOUT: usize = mem::size_of::<i32>();
    /// Offset of `nfds` field.
    pub const OFFSET_OF_NFDS: usize = 0;
    /// Offset of `timeout` field.
    pub const OFFSET_OF_TIMEOUT: usize = Self::OFFSET_OF_NFDS + Self::SIZE_OF_NFDS;
    /// Offset where the array of file descriptors starts.
    pub const OFFSET_OF_FDS: usize = Self::OFFSET_OF_TIMEOUT + Self::SIZE_OF_TIMEOUT;

    /// Maximum size of the message.
    pub const MAX_SIZE: usize =
        Self::OFFSET_OF_FDS + OPEN_MAX * (mem::size_of::<i32>() + mem::size_of::<i16>());

    ///
    /// # Description
    ///
    /// Creates a new request for the `poll()` system call.
    ///
    /// # Parameters
    ///
    /// - `fds`: File descriptors to poll.
    /// - `events`: Events to poll for on each file descriptor.
    /// - `timeout`: Timeout for the poll operation, in milliseconds.
    ///
    /// # Return Value
    ///
    /// On success, this function returns the newly created request message for the `poll()` system
    /// call. Otherwise, it returns an error object that describes the failure.
    ///
    pub fn new(fds: &[i32], events: &[i16], timeout: i32) -> Result<Self, Error> {
        // Check if array of file descriptors is empty.
        if fds.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument, "no file descriptors"));
        }

        // Check if array of file descriptors is too large.
        if fds.len() > OPEN_MAX {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of file descriptors exceeds maximum supported",
            ));
        }

        // Attempt to convert `fds.len()` as a `u8`.
        let nfds: u8 = match fds.len().try_into() {
            Ok(nfds) => nfds,
            Err(_error) => {
                return Err(Error::new(
                    ErrorCode::ValueOutOfRange,
                    "cannot encode number of file descriptors",
                ));
            },
        };

        // Check if array of events is empty.
        if events.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument, "no events"));
        }

        // Check if array of events is too large.
        if events.len() > OPEN_MAX {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of events exceeds maximum supported",
            ));
        }

        // Check if the number of events does not match the number of file descriptors.
        if fds.len() != events.len() {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of events does not match number of file descriptors",
            ));
        }

        Ok(Self {
            nfds,
            timeout,
            fds: fds.to_vec(),
            events: events.to_vec(),
        })
    }
}

impl MessageSerializer for PollRequest {
    /// Serializes a request message for the `poll()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        // Allocate buffer.
        let mut buffer: Vec<u8> = Vec::new();

        // Serialize `nfds` field.
        buffer.push(self.nfds);
        // Serialize `timeout` field.
        buffer.extend_from_slice(&self.timeout.to_le_bytes());
        // Serialize `fds` array.
        for fd in &self.fds {
            buffer.extend_from_slice(&fd.to_le_bytes());
        }
        // Serialize `events` array.
        for ev in &self.events {
            buffer.extend_from_slice(&ev.to_le_bytes());
        }

        buffer
    }
}

impl MessageDeserializer for PollRequest {
    /// Deserializes a request message for the `poll()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the buffer is too short.
        if bytes.len() < Self::OFFSET_OF_FDS {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer is too short"));
        }

        // Check if message is too long.
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too long"));
        }

        // Deserialize `nfds` field.
        let nfds: u8 = bytes[Self::OFFSET_OF_NFDS];
        if nfds == 0 || nfds as usize > OPEN_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "invalid nfds"));
        }

        // Deserialize `timeout` field.
        let timeout: i32 = i32::from_le_bytes(
            bytes[Self::OFFSET_OF_TIMEOUT..Self::OFFSET_OF_FDS]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid timeout"))?,
        );

        // Check if the buffer is too short to hold all fds and events.
        let fds_size: usize = nfds as usize * mem::size_of::<i32>();
        let events_size: usize = nfds as usize * mem::size_of::<i16>();
        if bytes.len() < Self::OFFSET_OF_FDS + fds_size + events_size {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer is too short"));
        }

        // Deserialize `fds` array.
        let mut fds: Vec<i32> = Vec::with_capacity(nfds.into());
        let mut events: Vec<i16> = Vec::with_capacity(nfds.into());
        let fds_offset: usize = Self::OFFSET_OF_FDS;
        for i in 0..nfds as usize {
            let base: usize = fds_offset + i * mem::size_of::<i32>();
            let fd: i32 = i32::from_le_bytes(
                bytes[base..base + mem::size_of::<i32>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid fd"))?,
            );
            fds.push(fd);
        }

        // Deserialize `events` array.
        let events_offset: usize = fds_offset + fds_size;
        for i in 0..nfds as usize {
            let base: usize = events_offset + i * mem::size_of::<i16>();
            let ev: i16 = i16::from_le_bytes(
                bytes[base..base + mem::size_of::<i16>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid event"))?,
            );
            events.push(ev);
        }

        PollRequest::new(&fds, &events, timeout)
    }
}

impl MessagePartitioner for PollRequest {
    /// Creates a new message request part for the `poll()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_request(
            tid,
            LinuxDaemonMessageHeader::PollRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}

//==================================================================================================
// PollResponse
//==================================================================================================

#[derive(Debug)]
pub struct PollResponse {
    /// Number of file descriptors with ready events.
    pub nready: u8,
    /// File descriptors that are ready.
    pub fds: Vec<i32>,
    /// Events that occurred on each ready file descriptor.
    pub revents: Vec<i16>,
}

impl PollResponse {
    /// Offset of `nready`.
    pub const OFFSET_OF_NREADY: usize = 0;
    /// Offset of first array (fds) after nready.
    pub const OFFSET_OF_FDS: usize = Self::OFFSET_OF_NREADY + mem::size_of::<u8>();

    /// Maximum size of the message.
    pub const MAX_SIZE: usize = Self::OFFSET_OF_FDS
        + OPEN_MAX * (core::mem::size_of::<i32>() + core::mem::size_of::<i16>());

    ///
    /// # Description
    ///
    /// Creates a new response for the `poll()` system call.
    ///
    /// # Parameters
    ///
    /// - `fds`: File descriptors that are ready.
    /// - `revents`: Events that occurred on each ready file descriptor.
    ///
    /// # Return Value
    ///
    /// On success, this function returns the newly created response message for the `poll()` system
    /// call. Otherwise, it returns an error object that describes the failure.
    ///
    pub fn new(fds: &[i32], revents: &[i16]) -> Result<Self, Error> {
        // Check if array of file descriptors is too large.
        if fds.len() > OPEN_MAX {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of file descriptors exceeds maximum supported",
            ));
        }

        // Check if array of events is too large.
        if revents.len() > OPEN_MAX {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of events exceeds maximum supported",
            ));
        }

        // Check if the number of events does not match the number of file descriptors.
        if fds.len() != revents.len() {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "fds and revents must have the same length",
            ));
        }

        // Attempt to convert `fds.len()` as a `u8`.
        let nready: u8 = match fds.len().try_into() {
            Ok(nready) => nready,
            Err(_error) => {
                return Err(Error::new(
                    ErrorCode::ValueOutOfRange,
                    "cannot encode number of ready file descriptors",
                ));
            },
        };

        Ok(Self {
            nready,
            fds: fds.to_vec(),
            revents: revents.to_vec(),
        })
    }
}

impl MessageSerializer for PollResponse {
    /// Serializes a response message for the `poll()` system call.
    fn to_bytes(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.push(self.nready);
        for fd in &self.fds {
            buffer.extend_from_slice(&fd.to_le_bytes());
        }
        for ev in &self.revents {
            buffer.extend_from_slice(&ev.to_le_bytes());
        }
        buffer
    }
}

impl MessageDeserializer for PollResponse {
    /// Deserializes a response message for the `poll()` system call.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the buffer is too short.
        if bytes.len() < Self::OFFSET_OF_FDS {
            // need at least nready
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer is too short"));
        }

        // Check if message is too long.
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(ErrorCode::InvalidMessage, "message is too long"));
        }

        // Deserialize `nready` field.
        let nready: u8 = bytes[Self::OFFSET_OF_NREADY];
        if nready as usize > OPEN_MAX {
            return Err(Error::new(ErrorCode::InvalidMessage, "invalid nready"));
        }

        // Check if the buffer is too short to hold all fds and events.
        let fds_size: usize = nready as usize * mem::size_of::<i32>();
        let events_size: usize = nready as usize * mem::size_of::<i16>();
        if bytes.len() < Self::OFFSET_OF_FDS + fds_size + events_size {
            return Err(Error::new(ErrorCode::InvalidMessage, "buffer is too short"));
        }

        // Deserialize `fds` array.
        let mut fds: Vec<i32> = Vec::with_capacity(nready as usize);
        let mut revents: Vec<i16> = Vec::with_capacity(nready as usize);
        let fds_offset: usize = Self::OFFSET_OF_FDS;
        for i in 0..nready as usize {
            let base: usize = fds_offset + i * mem::size_of::<i32>();
            let fd: i32 = i32::from_le_bytes(
                bytes[base..base + mem::size_of::<i32>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid fd"))?,
            );
            fds.push(fd);
        }

        // Deserialize `revents` array.
        let events_offset: usize = fds_offset + fds_size;
        for i in 0..nready as usize {
            let base: usize = events_offset + i * mem::size_of::<i16>();
            let ev: i16 = i16::from_le_bytes(
                bytes[base..base + mem::size_of::<i16>()]
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::InvalidMessage, "invalid event"))?,
            );
            revents.push(ev);
        }

        PollResponse::new(&fds, &revents)
    }
}

impl MessagePartitioner for PollResponse {
    /// Creates a new response message part for the `poll()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; LinuxDaemonMessagePart::PAYLOAD_SIZE],
    ) -> Result<Message, Error> {
        LinuxDaemonMessagePart::build_response(
            tid,
            LinuxDaemonMessageHeader::PollResponsePart,
            total_parts,
            part_number,
            payload_size,
            payload,
        )
    }
}
