// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::{
        MessageDeserializer,
        MessagePartitioner,
        MessageSerializer,
        SystemCallMessagePart,
    },
    SystemCallMessage,
    SystemCallMessageHeader,
};
use ::alloc::vec::Vec;
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
use ::sysapi::sys_select::{
    fd_set,
    timeval,
    FD_SETSIZE,
};

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Encodes an optional [`fd_set`] at `offset` as a one-byte presence flag (`0` = absent,
/// `1` = present) followed by the set's fixed-width wire payload. A `None` value leaves the
/// zero-initialized flag and payload untouched.
fn encode_optional_fd_set(bytes: &mut [u8], offset: usize, value: Option<fd_set>) {
    if let Some(set) = value {
        bytes[offset] = 1;
        bytes[offset + 1..offset + 1 + fd_set::WIRE_SIZE].copy_from_slice(&set.to_bytes());
    }
}

/// Decodes an optional [`fd_set`] encoded by [`encode_optional_fd_set()`].
fn decode_optional_fd_set(bytes: &[u8], offset: usize) -> Result<Option<fd_set>, Error> {
    match bytes[offset] {
        0 => Ok(None),
        1 => {
            let mut payload: [u8; fd_set::WIRE_SIZE] = [0; fd_set::WIRE_SIZE];
            payload.copy_from_slice(&bytes[offset + 1..offset + 1 + fd_set::WIRE_SIZE]);
            Ok(Some(fd_set::from_bytes(payload)))
        },
        _ => Err(Error::new(
            ErrorCode::InvalidMessage,
            "invalid fd_set presence flag in select message",
        )),
    }
}

//==================================================================================================
// SelectRequest
//==================================================================================================

/// Request message for the `select()` system call.
///
/// The payload is serialized with an explicit, offset-based wire layout (rather than relying on
/// the in-memory representation of `Option<...>` fields) so that a 32-bit guest and a 64-bit host
/// interoperate byte-for-byte:
///
/// `nfds (1) | readfds (1 + fd_set::WIRE_SIZE) | writefds (1 + fd_set::WIRE_SIZE) |
///  errorfds (1 + fd_set::WIRE_SIZE) | timeout (1 + timeval::WIRE_SIZE)`.
///
/// Each optional field is prefixed by a one-byte presence flag (`0` = absent, `1` = present).
///
/// The serialized form (exactly [`Self::WIRE_SIZE`] bytes) is transported as a variable-length,
/// multi-part `SelectRequestPart` stream — the same pattern used by the path-bearing requests
/// (`openat`, `renameat`, …) — rather than a single fixed-size [`SystemCallMessage`].
#[derive(Debug)]
pub struct SelectRequest {
    /// Number of file descriptors in each set (must be <= FD_SETSIZE).
    pub nfds: u8,
    /// Read file descriptors of interest.
    pub readfds: Option<fd_set>,
    /// Write file descriptors of interest.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors of interest.
    pub errorfds: Option<fd_set>,
    /// Timeout, encoded in `timeval`'s fixed-width wire format for cross-architecture IPC.
    pub timeout: Option<[u8; timeval::WIRE_SIZE]>,
}

// Ensure that the maximum number of file descriptors can be encoded in a `u8`.
::static_assert::assert_eq!(FD_SETSIZE < u8::MAX as usize);

impl SelectRequest {
    /// Wire size of an optional `fd_set` field (presence flag + set payload).
    const OPTIONAL_FD_SET_WIRE_SIZE: usize = mem::size_of::<u8>() + fd_set::WIRE_SIZE;
    /// Wire size of the optional timeout field (presence flag + timeval payload).
    const OPTIONAL_TIMEOUT_WIRE_SIZE: usize = mem::size_of::<u8>() + timeval::WIRE_SIZE;

    /// Offset of the `nfds` field.
    const OFFSET_NFDS: usize = 0;
    /// Offset of the read file descriptor set.
    const OFFSET_READFDS: usize = Self::OFFSET_NFDS + mem::size_of::<u8>();
    /// Offset of the write file descriptor set.
    const OFFSET_WRITEFDS: usize = Self::OFFSET_READFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the error file descriptor set.
    const OFFSET_ERRORFDS: usize = Self::OFFSET_WRITEFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the timeout field.
    const OFFSET_TIMEOUT: usize = Self::OFFSET_ERRORFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Total number of meaningful wire bytes.
    const WIRE_SIZE: usize = Self::OFFSET_TIMEOUT + Self::OPTIONAL_TIMEOUT_WIRE_SIZE;

    /// Maximum size of the serialized request.
    ///
    /// The wire layout is fixed-width, so the maximum size equals [`Self::WIRE_SIZE`]. The
    /// serialized form is transported as a `SelectRequestPart` stream bounded by
    /// [`SystemCallMessagePart::PAYLOAD_SIZE`], so it is not constrained by the single-message
    /// [`SystemCallMessage::PAYLOAD_SIZE`].
    pub const MAX_SIZE: usize = Self::WIRE_SIZE;

    /// Creates a new `SelectRequest`.
    ///
    /// Validates that `nfds` does not exceed [`FD_SETSIZE`] and fits in a `u8`, then copies the
    /// referenced descriptor sets and timeout into an owned request.
    pub fn new(
        nfds: usize,
        readfds: &Option<&mut fd_set>,
        writefds: &Option<&mut fd_set>,
        errorfds: &Option<&mut fd_set>,
        timeout: &Option<timeval>,
    ) -> Result<Self, Error> {
        // Validate number of file descriptors.
        if nfds > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "number of file descriptors exceeds maximum supported",
            ));
        }

        // Attempt to encode nfds as u8 (should always succeed due to static assert, but be safe).
        let nfds: u8 = match nfds.try_into() {
            Ok(v) => v,
            Err(_e) => {
                return Err(Error::new(
                    ErrorCode::ValueOutOfRange,
                    "cannot encode number of file descriptors",
                ));
            },
        };

        Ok(Self {
            nfds,
            readfds: readfds.as_ref().map(|fd_set| **fd_set),
            writefds: writefds.as_ref().map(|fd_set| **fd_set),
            errorfds: errorfds.as_ref().map(|fd_set| **fd_set),
            timeout: timeout.as_ref().map(|tv| tv.to_bytes()),
        })
    }
}

impl MessageSerializer for SelectRequest {
    /// Serializes the request into its fixed-width wire representation.
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = ::alloc::vec![0u8; Self::WIRE_SIZE];
        bytes[Self::OFFSET_NFDS] = self.nfds;
        encode_optional_fd_set(&mut bytes, Self::OFFSET_READFDS, self.readfds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_WRITEFDS, self.writefds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_ERRORFDS, self.errorfds);
        if let Some(payload) = self.timeout {
            bytes[Self::OFFSET_TIMEOUT] = 1;
            bytes[Self::OFFSET_TIMEOUT + 1..Self::OFFSET_TIMEOUT + 1 + timeval::WIRE_SIZE]
                .copy_from_slice(&payload);
        }
        bytes
    }
}

impl MessageDeserializer for SelectRequest {
    /// Deserializes a request from its fixed-width wire representation.
    fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // The wire layout is fixed-width, so the reassembled buffer must be exactly `WIRE_SIZE`
        // bytes (== `MAX_SIZE`). Reject anything shorter (would overflow the field slices) or
        // longer (unexpected trailing bytes).
        if bytes.len() < Self::WIRE_SIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "select request message is too short",
            ));
        }
        if bytes.len() > Self::MAX_SIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "select request message is too long",
            ));
        }

        let nfds: u8 = bytes[Self::OFFSET_NFDS];
        // Reject messages whose number of file descriptors exceeds the wire contract bound.
        if nfds as usize > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "number of file descriptors exceeds maximum supported in select message",
            ));
        }
        let readfds: Option<fd_set> = decode_optional_fd_set(bytes, Self::OFFSET_READFDS)?;
        let writefds: Option<fd_set> = decode_optional_fd_set(bytes, Self::OFFSET_WRITEFDS)?;
        let errorfds: Option<fd_set> = decode_optional_fd_set(bytes, Self::OFFSET_ERRORFDS)?;
        let timeout: Option<[u8; timeval::WIRE_SIZE]> = match bytes[Self::OFFSET_TIMEOUT] {
            0 => None,
            1 => {
                let mut payload: [u8; timeval::WIRE_SIZE] = [0; timeval::WIRE_SIZE];
                payload.copy_from_slice(
                    &bytes[Self::OFFSET_TIMEOUT + 1..Self::OFFSET_TIMEOUT + 1 + timeval::WIRE_SIZE],
                );
                Some(payload)
            },
            _ => {
                return Err(Error::new(
                    ErrorCode::InvalidMessage,
                    "invalid timeout presence flag in select message",
                ));
            },
        };
        Ok(Self {
            nfds,
            readfds,
            writefds,
            errorfds,
            timeout,
        })
    }
}

impl MessagePartitioner for SelectRequest {
    /// Creates a new message request part for the `select()` system call.
    fn new_part(
        tid: ThreadIdentifier,
        total_parts: u16,
        part_number: u16,
        payload_size: u8,
        payload: [u8; SystemCallMessagePart::PAYLOAD_SIZE],
        destination: ProcessIdentifier,
        message_type: MessageType,
    ) -> Result<Message, Error> {
        SystemCallMessagePart::build_request(
            tid,
            SystemCallMessageHeader::SelectRequestPart,
            total_parts,
            part_number,
            payload_size,
            payload,
            destination,
            message_type,
        )
    }
}

//==================================================================================================
// SelectResponse
//==================================================================================================

/// Response message for the `select()` system call.
///
/// Uses the same explicit, offset-based wire layout as [`SelectRequest`] for cross-architecture
/// IPC compatibility:
///
/// `ready_fds (1) | readfds (1 + fd_set::WIRE_SIZE) | writefds (1 + fd_set::WIRE_SIZE) |
///  errorfds (1 + fd_set::WIRE_SIZE)` followed by padding.
///
/// Each optional field is prefixed by a one-byte presence flag (`0` = absent, `1` = present).
#[derive(Debug)]
pub struct SelectResponse {
    /// Number of file descriptors ready.
    pub ready_fds: u8,
    /// Read file descriptors ready.
    pub readfds: Option<fd_set>,
    /// Write file descriptors ready.
    pub writefds: Option<fd_set>,
    /// Error/exception file descriptors ready.
    pub errorfds: Option<fd_set>,
}

// Ensure the fixed-width wire layout fits within the payload, so the manual offsets/slices in
// `from_bytes()`/`into_bytes()` cannot overflow `PAYLOAD_SIZE` and panic at runtime.
::static_assert::assert_eq!(SelectResponse::WIRE_SIZE <= SystemCallMessage::PAYLOAD_SIZE);

impl SelectResponse {
    /// Wire size of an optional `fd_set` field (presence flag + set payload).
    const OPTIONAL_FD_SET_WIRE_SIZE: usize = mem::size_of::<u8>() + fd_set::WIRE_SIZE;

    /// Offset of the `ready_fds` field.
    const OFFSET_READY_FDS: usize = 0;
    /// Offset of the read file descriptor set.
    const OFFSET_READFDS: usize = Self::OFFSET_READY_FDS + mem::size_of::<u8>();
    /// Offset of the write file descriptor set.
    const OFFSET_WRITEFDS: usize = Self::OFFSET_READFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Offset of the error file descriptor set.
    const OFFSET_ERRORFDS: usize = Self::OFFSET_WRITEFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;
    /// Total number of meaningful wire bytes.
    const WIRE_SIZE: usize = Self::OFFSET_ERRORFDS + Self::OPTIONAL_FD_SET_WIRE_SIZE;

    /// Creates a new `SelectResponse`.
    fn new(
        ready_fds: u8,
        readfds: &Option<fd_set>,
        writefds: &Option<fd_set>,
        errorfds: &Option<fd_set>,
    ) -> Self {
        Self {
            ready_fds,
            readfds: *readfds,
            writefds: *writefds,
            errorfds: *errorfds,
        }
    }

    /// Deserializes a response from its fixed-width wire representation.
    pub fn from_bytes(bytes: [u8; SystemCallMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        let ready_fds: u8 = bytes[Self::OFFSET_READY_FDS];
        // Reject messages whose number of ready file descriptors exceeds the wire contract bound.
        if ready_fds as usize > FD_SETSIZE {
            return Err(Error::new(
                ErrorCode::InvalidMessage,
                "number of ready file descriptors exceeds maximum supported in select message",
            ));
        }
        let readfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_READFDS)?;
        let writefds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_WRITEFDS)?;
        let errorfds: Option<fd_set> = decode_optional_fd_set(&bytes, Self::OFFSET_ERRORFDS)?;
        Ok(Self {
            ready_fds,
            readfds,
            writefds,
            errorfds,
        })
    }

    /// Serializes the response into its fixed-width wire representation.
    fn into_bytes(self) -> [u8; SystemCallMessage::PAYLOAD_SIZE] {
        let mut bytes: [u8; SystemCallMessage::PAYLOAD_SIZE] = [0; SystemCallMessage::PAYLOAD_SIZE];
        bytes[Self::OFFSET_READY_FDS] = self.ready_fds;
        encode_optional_fd_set(&mut bytes, Self::OFFSET_READFDS, self.readfds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_WRITEFDS, self.writefds);
        encode_optional_fd_set(&mut bytes, Self::OFFSET_ERRORFDS, self.errorfds);
        bytes
    }

    /// Builds a kernel IPC message for a `select()` system call response.
    pub fn build(
        tid: ThreadIdentifier,
        ready_fds: u8,
        readfds: &Option<fd_set>,
        writefds: &Option<fd_set>,
        errorfds: &Option<fd_set>,
        source: ProcessIdentifier,
        message_type: MessageType,
    ) -> Message {
        let message: SelectResponse = SelectResponse::new(ready_fds, readfds, writefds, errorfds);
        let message: SystemCallMessage =
            SystemCallMessage::new(SystemCallMessageHeader::SelectResponse, message.into_bytes());
        let message: Message = Message::new(
            MessageSender::from(source),
            MessageReceiver::from(tid),
            message_type,
            None,
            message.into_bytes(),
        );
        message
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Thread identifier used as the message sender in the part-transport round trips.
    const TEST_TID: i32 = 1;
    /// Destination process identifier used in the part-transport round trips.
    const TEST_DEST: i32 = 2;

    /// Reassembles the `SystemCallMessagePart`s carried by the IPC messages produced by
    /// `into_parts()`, asserting every part is tagged as a `SelectRequestPart`.
    fn extract_parts(messages: Vec<Message>) -> Vec<SystemCallMessagePart> {
        messages
            .into_iter()
            .map(|msg| {
                let daemon_msg: SystemCallMessage = SystemCallMessage::try_from_bytes(msg.payload)
                    .expect("valid SystemCallMessage");
                // Copy the header out of the packed message before comparing it.
                let header: SystemCallMessageHeader = daemon_msg.header;
                assert_eq!(
                    header,
                    SystemCallMessageHeader::SelectRequestPart,
                    "unexpected part header"
                );
                SystemCallMessagePart::from_bytes(daemon_msg.payload)
            })
            .collect()
    }

    /// Builds an `fd_set` with the given file descriptors set.
    fn fd_set_with(bits: &[usize]) -> fd_set {
        let mut set: fd_set = fd_set::default();
        for &fd in bits {
            set.set_bit(fd).expect("fd within range");
        }
        set
    }

    /// Serializes `request`, splits it into parts, reassembles them, and asserts the decoded
    /// request matches the original byte-for-byte.
    fn assert_round_trip(request: SelectRequest) {
        // Snapshot the request fields before the value is consumed by `into_parts()`.
        let nfds: u8 = request.nfds;
        let readfds: Option<[u8; fd_set::WIRE_SIZE]> = request.readfds.map(|s| s.to_bytes());
        let writefds: Option<[u8; fd_set::WIRE_SIZE]> = request.writefds.map(|s| s.to_bytes());
        let errorfds: Option<[u8; fd_set::WIRE_SIZE]> = request.errorfds.map(|s| s.to_bytes());
        let timeout: Option<[u8; timeval::WIRE_SIZE]> = request.timeout;

        // The fixed-width wire layout always serializes to exactly `WIRE_SIZE` bytes.
        assert_eq!(request.to_bytes().len(), SelectRequest::WIRE_SIZE, "unexpected wire size");

        // Split into a `SelectRequestPart` stream and reassemble it.
        let messages: Vec<Message> = request
            .into_parts(
                ThreadIdentifier::from(TEST_TID),
                ProcessIdentifier::from(TEST_DEST),
                MessageType::Ikc,
            )
            .expect("partition should succeed");

        // A 45-byte request spans two parts on the current ABI; this exercises the part boundary.
        let expected_parts: usize =
            SelectRequest::MAX_SIZE.div_ceil(SystemCallMessagePart::PAYLOAD_SIZE);
        assert_eq!(messages.len(), expected_parts, "unexpected number of parts");

        let parts: Vec<SystemCallMessagePart> = extract_parts(messages);
        let decoded: SelectRequest =
            SelectRequest::from_parts(&parts).expect("reassembly should succeed");

        assert_eq!(decoded.nfds, nfds, "nfds mismatch");
        assert_eq!(decoded.readfds.map(|s| s.to_bytes()), readfds, "readfds mismatch");
        assert_eq!(decoded.writefds.map(|s| s.to_bytes()), writefds, "writefds mismatch");
        assert_eq!(decoded.errorfds.map(|s| s.to_bytes()), errorfds, "errorfds mismatch");
        assert_eq!(decoded.timeout, timeout, "timeout mismatch");
    }

    /// Round-trips a request with every descriptor set and the timeout present.
    #[test]
    fn round_trip_all_sets_present() {
        let mut readfds: fd_set = fd_set_with(&[0, 3, 7]);
        let mut writefds: fd_set = fd_set_with(&[1, 2]);
        let mut errorfds: fd_set = fd_set_with(&[FD_SETSIZE - 1]);
        let timeout: timeval = timeval {
            tv_sec: 5,
            tv_usec: 123,
        };
        let request: SelectRequest = SelectRequest::new(
            8,
            &Some(&mut readfds),
            &Some(&mut writefds),
            &Some(&mut errorfds),
            &Some(timeout),
        )
        .expect("valid request");
        assert_round_trip(request);
    }

    /// Round-trips a request with every optional field absent.
    #[test]
    fn round_trip_all_sets_absent() {
        let request: SelectRequest =
            SelectRequest::new(0, &None, &None, &None, &None).expect("valid request");
        assert_round_trip(request);
    }

    /// `new()` rejects an `nfds` greater than `FD_SETSIZE`.
    #[test]
    fn new_rejects_nfds_above_setsize() {
        assert!(
            SelectRequest::new(FD_SETSIZE + 1, &None, &None, &None, &None).is_err(),
            "nfds above FD_SETSIZE must be rejected"
        );
    }

    /// `new()` accepts the maximum representable descriptor set size.
    #[test]
    fn new_accepts_nfds_at_setsize() {
        let request: SelectRequest =
            SelectRequest::new(FD_SETSIZE, &None, &None, &None, &None).expect("valid request");
        assert_eq!(request.nfds, FD_SETSIZE as u8, "nfds mismatch");
    }

    /// `try_from_bytes()` rejects a buffer shorter than the fixed wire layout.
    #[test]
    fn try_from_bytes_rejects_short_buffer() {
        let short: Vec<u8> = ::alloc::vec![0u8; SelectRequest::WIRE_SIZE - 1];
        assert!(SelectRequest::try_from_bytes(&short).is_err(), "short buffer must be rejected");
    }

    /// `try_from_bytes()` rejects unexpected trailing bytes.
    #[test]
    fn try_from_bytes_rejects_long_buffer() {
        let long: Vec<u8> = ::alloc::vec![0u8; SelectRequest::WIRE_SIZE + 1];
        assert!(SelectRequest::try_from_bytes(&long).is_err(), "long buffer must be rejected");
    }
}
