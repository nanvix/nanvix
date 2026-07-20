// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt;
use ::sys::{
    ipc::{
        Message,
        SG_BULK_MAX_BYTES,
    },
    pm::ThreadIdentifier,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Size, in bytes, of a serialized [`Message`] on the wire.
pub const MESSAGE_BYTES: usize = ::core::mem::size_of::<Message>();

/// Number of bytes in the little-endian `u32` length prefix that precedes every wire frame body.
pub const LENGTH_PREFIX_BYTES: usize = ::core::mem::size_of::<u32>();

/// Number of bytes in a wire operation discriminator.
const OP_BYTES: usize = ::core::mem::size_of::<WireOp>();

/// Number of bytes in a wire presence flag.
const PRESENCE_BYTES: usize = ::core::mem::size_of::<Presence>();

/// Number of bytes in a serialized thread identifier.
const TID_BYTES: usize = ::core::mem::size_of::<i32>();

/// Maximum size, in bytes, of a bulk payload embedded in a wire frame.
///
/// This matches the guest scatter/gather transfer ceiling so the decoupled networkd transport
/// cannot accept a bulk payload that the UserVM scatter/gather path would reject.
pub const MAX_PAYLOAD_BYTES: usize = SG_BULK_MAX_BYTES;

/// Maximum metadata overhead, in bytes, of any wire frame body.
///
/// The largest frame metadata appears in [`NetworkResult::Pull`]: an op discriminator, explicit
/// response `tid`, embedded [`Message`], and a `u32` bulk payload length.
const MAX_FRAME_METADATA_BYTES: usize = OP_BYTES + TID_BYTES + MESSAGE_BYTES + LENGTH_PREFIX_BYTES;

/// Maximum size, in bytes, of a wire frame body.
///
/// Frames larger than this are rejected as a protocol violation so that a corrupt or hostile peer
/// cannot force an allocation larger than the largest valid payload-bearing frame.
pub const MAX_FRAME_BYTES: usize = MAX_PAYLOAD_BYTES + MAX_FRAME_METADATA_BYTES;

//==================================================================================================
// WireError
//==================================================================================================

///
/// # Description
///
/// Error produced while encoding or decoding a networkd wire frame.
///
#[derive(Debug, PartialEq, Eq)]
pub enum WireError {
    /// The frame ended before all expected fields were read.
    Truncated,
    /// The frame body exceeded [`MAX_FRAME_BYTES`].
    FrameTooLarge(usize),
    /// A bulk payload exceeded [`MAX_PAYLOAD_BYTES`].
    PayloadTooLarge(usize),
    /// The operation discriminator byte was not recognized.
    InvalidOp(u8),
    /// The presence flag for an optional field was not recognized.
    InvalidPresence(u8),
    /// The embedded [`Message`] bytes could not be decoded.
    InvalidMessage,
    /// Trailing bytes remained after decoding a complete frame.
    TrailingBytes,
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::Truncated => write!(f, "wire frame truncated"),
            WireError::FrameTooLarge(len) => {
                write!(f, "wire frame too large: {len} bytes (max {MAX_FRAME_BYTES})")
            },
            WireError::PayloadTooLarge(len) => {
                write!(f, "wire payload too large: {len} bytes (max {MAX_PAYLOAD_BYTES})")
            },
            WireError::InvalidOp(op) => write!(f, "invalid wire op discriminator: {op}"),
            WireError::InvalidPresence(p) => write!(f, "invalid wire presence flag: {p}"),
            WireError::InvalidMessage => write!(f, "invalid embedded message bytes"),
            WireError::TrailingBytes => write!(f, "trailing bytes after wire frame"),
        }
    }
}

impl ::std::error::Error for WireError {}

//==================================================================================================
// WireOp
//==================================================================================================

/// Operation discriminator encoded at the start of each wire frame body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum WireOp {
    /// Inline networking message (`connect`, `socket`, ...).
    Message = 1,
    /// Scatter/gather push request or response (`send` or `sendto`).
    Push = 2,
    /// Scatter/gather pull request or response (`recv` or `recvfrom`).
    Pull = 3,
}

impl WireOp {
    fn to_u8(self) -> u8 {
        match self {
            Self::Message => 1,
            Self::Push => 2,
            Self::Pull => 3,
        }
    }
}

impl TryFrom<u8> for WireOp {
    type Error = WireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Message),
            2 => Ok(Self::Push),
            3 => Ok(Self::Pull),
            other => Err(WireError::InvalidOp(other)),
        }
    }
}

//==================================================================================================
// Presence
//==================================================================================================

/// Presence flag used to encode optional fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Presence {
    /// Value is absent (`None`).
    None = 0,
    /// Value is present (`Some`).
    Some = 1,
}

impl Presence {
    fn to_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Some => 1,
        }
    }
}

impl TryFrom<u8> for Presence {
    type Error = WireError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Some),
            other => Err(WireError::InvalidPresence(other)),
        }
    }
}

//==================================================================================================
// NetworkRequest
//==================================================================================================

///
/// # Description
///
/// A request forwarded from the user VM to a decoupled `networkd` process.
///
/// Every request carries the original [`Message`] plus, for scatter/gather push operations, the
/// already-drained bulk payload. The server reconstructs the `source` and decoded system-call
/// message from the embedded [`Message`] exactly as the in-process handler does, so no additional
/// metadata needs to cross the wire. The originating guest thread identifier — used to correlate
/// the eventual response — is already carried inside the embedded [`Message`], so requests need no
/// explicit correlation field.
///
#[derive(Debug)]
pub struct NetworkRequest {
    /// The requested operation and its owned buffers.
    pub op: NetworkOp,
}

///
/// # Description
///
/// The operation carried by a [`NetworkRequest`].
///
#[derive(Debug)]
pub enum NetworkOp {
    /// An inline networking message (e.g. `connect` or `socket`).
    Message(Message),
    /// A `send` or `sendto` request whose bulk payload has been drained from the transport.
    Push {
        /// The original request message.
        msg: Message,
        /// The payload bytes to send.
        data: Vec<u8>,
    },
    /// A `recv` or `recvfrom` request.
    Pull(Message),
}

impl NetworkOp {
    ///
    /// # Description
    ///
    /// Returns the originating guest thread identifier carried by this operation's embedded
    /// [`Message`].
    ///
    /// The `tid` correlates the eventual response with the waiting caller, so it is read before the
    /// operation is consumed by dispatch.
    ///
    pub fn tid(&self) -> ThreadIdentifier {
        match self {
            NetworkOp::Message(msg) | NetworkOp::Pull(msg) | NetworkOp::Push { msg, .. } => {
                msg.source.tid
            },
        }
    }
}

impl NetworkRequest {
    ///
    /// # Description
    ///
    /// Serializes this request into the body of a wire frame (without the length prefix).
    ///
    /// # Errors
    ///
    /// Returns [`WireError::PayloadTooLarge`] if a bulk payload exceeds [`MAX_PAYLOAD_BYTES`], or
    /// [`WireError::FrameTooLarge`] if the encoded body exceeds [`MAX_FRAME_BYTES`].
    ///
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let encoded_len: usize = self.encoded_len()?;
        let mut out: Vec<u8> = Vec::with_capacity(encoded_len);
        match &self.op {
            NetworkOp::Message(msg) => {
                write_op(&mut out, WireOp::Message);
                write_message(&mut out, msg);
            },
            NetworkOp::Push { msg, data } => {
                write_op(&mut out, WireOp::Push);
                write_message(&mut out, msg);
                write_bytes(&mut out, data)?;
            },
            NetworkOp::Pull(msg) => {
                write_op(&mut out, WireOp::Pull);
                write_message(&mut out, msg);
            },
        }
        debug_assert_eq!(out.len(), encoded_len);
        Ok(out)
    }

    fn encoded_len(&self) -> Result<usize, WireError> {
        match &self.op {
            NetworkOp::Message(_) | NetworkOp::Pull(_) => {
                checked_frame_len(&[OP_BYTES, MESSAGE_BYTES])
            },
            NetworkOp::Push { data, .. } => {
                ensure_payload_len(data.len())?;
                checked_frame_len(&[OP_BYTES, MESSAGE_BYTES, LENGTH_PREFIX_BYTES, data.len()])
            },
        }
    }

    ///
    /// # Description
    ///
    /// Decodes a request from the body of a wire frame (without the length prefix).
    ///
    /// # Errors
    ///
    /// Returns [`WireError::FrameTooLarge`] if `bytes` exceeds [`MAX_FRAME_BYTES`].
    ///
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        ensure_frame_len(bytes.len())?;
        let mut cursor: Cursor<'_> = Cursor::new(bytes);
        let op: WireOp = cursor.read_op()?;
        let op: NetworkOp = match op {
            WireOp::Message => NetworkOp::Message(cursor.read_message()?),
            WireOp::Push => {
                let msg: Message = cursor.read_message()?;
                let data: Vec<u8> = cursor.read_bytes()?;
                NetworkOp::Push { msg, data }
            },
            WireOp::Pull => NetworkOp::Pull(cursor.read_message()?),
        };
        cursor.finish()?;
        Ok(Self { op })
    }

    ///
    /// # Description
    ///
    /// Serializes this request into a complete, length-prefixed wire frame ready to be written to
    /// the transport.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`NetworkRequest::encode`].
    ///
    pub fn to_frame(&self) -> Result<Vec<u8>, WireError> {
        frame_body(&self.encode()?)
    }
}

//==================================================================================================
// NetworkResponse
//==================================================================================================

///
/// # Description
///
/// A response returned by a decoupled `networkd` process to the user VM.
///
/// Response variants mirror the return types of the network daemon's handlers and correlate with
/// their originating request via the guest thread identifier (`tid`). Correlating by `tid` works
/// because a guest thread issues at most one blocking networking call at a time; it is carried
/// explicitly here so that even an empty [`NetworkResult::Message(None)`] — which embeds no
/// [`Message`] — can be routed back to the waiting caller.
///
#[derive(Debug)]
pub struct NetworkResponse {
    /// Correlates this response with its originating request by the guest thread identifier.
    pub tid: ThreadIdentifier,
    /// The operation-specific result.
    pub result: NetworkResult,
}

///
/// # Description
///
/// The operation-specific payload carried by a [`NetworkResponse`].
///
#[derive(Debug)]
pub enum NetworkResult {
    /// Result of an inline [`NetworkOp::Message`]: zero or more response messages, or nothing.
    Message(Option<Vec<Message>>),
    /// Result of a [`NetworkOp::Push`]: a single response message.
    Push(Message),
    /// Result of a [`NetworkOp::Pull`]: a response message plus the received payload.
    Pull {
        /// The response message.
        msg: Message,
        /// The received payload bytes to push back to the guest.
        data: Vec<u8>,
    },
}

impl NetworkResponse {
    ///
    /// # Description
    ///
    /// Serializes this response into the body of a wire frame (without the length prefix).
    ///
    /// # Errors
    ///
    /// Returns [`WireError::PayloadTooLarge`] if a bulk payload exceeds [`MAX_PAYLOAD_BYTES`], or
    /// [`WireError::FrameTooLarge`] if the encoded body exceeds [`MAX_FRAME_BYTES`].
    ///
    pub fn encode(&self) -> Result<Vec<u8>, WireError> {
        let encoded_len: usize = self.encoded_len()?;
        let mut out: Vec<u8> = Vec::with_capacity(encoded_len);
        match &self.result {
            NetworkResult::Message(messages) => {
                write_op(&mut out, WireOp::Message);
                write_tid(&mut out, self.tid);
                match messages {
                    None => write_presence(&mut out, Presence::None),
                    Some(messages) => {
                        write_presence(&mut out, Presence::Some);
                        let count: u32 = message_count_to_u32(messages.len())?;
                        write_message_count(&mut out, count);
                        for msg in messages {
                            write_message(&mut out, msg);
                        }
                    },
                }
            },
            NetworkResult::Push(msg) => {
                write_op(&mut out, WireOp::Push);
                write_tid(&mut out, self.tid);
                write_message(&mut out, msg);
            },
            NetworkResult::Pull { msg, data } => {
                write_op(&mut out, WireOp::Pull);
                write_tid(&mut out, self.tid);
                write_message(&mut out, msg);
                write_bytes(&mut out, data)?;
            },
        }
        debug_assert_eq!(out.len(), encoded_len);
        Ok(out)
    }

    fn encoded_len(&self) -> Result<usize, WireError> {
        match &self.result {
            NetworkResult::Message(None) => {
                checked_frame_len(&[OP_BYTES, TID_BYTES, PRESENCE_BYTES])
            },
            NetworkResult::Message(Some(messages)) => {
                let messages_bytes: usize = messages_bytes_len(messages.len())?;
                checked_frame_len(&[
                    OP_BYTES,
                    TID_BYTES,
                    PRESENCE_BYTES,
                    LENGTH_PREFIX_BYTES,
                    messages_bytes,
                ])
            },
            NetworkResult::Push(_) => checked_frame_len(&[OP_BYTES, TID_BYTES, MESSAGE_BYTES]),
            NetworkResult::Pull { data, .. } => {
                ensure_payload_len(data.len())?;
                checked_frame_len(&[
                    OP_BYTES,
                    TID_BYTES,
                    MESSAGE_BYTES,
                    LENGTH_PREFIX_BYTES,
                    data.len(),
                ])
            },
        }
    }

    ///
    /// # Description
    ///
    /// Decodes a response from the body of a wire frame (without the length prefix).
    ///
    /// # Errors
    ///
    /// Returns [`WireError::FrameTooLarge`] if `bytes` exceeds [`MAX_FRAME_BYTES`].
    ///
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        ensure_frame_len(bytes.len())?;
        let mut cursor: Cursor<'_> = Cursor::new(bytes);
        let op: WireOp = cursor.read_op()?;
        let tid: ThreadIdentifier = cursor.read_tid()?;
        let result: NetworkResult = match op {
            WireOp::Message => match cursor.read_presence()? {
                Presence::None => NetworkResult::Message(None),
                Presence::Some => {
                    let count: u32 = cursor.read_message_count()?;
                    let count: usize = usize::try_from(count).map_err(|_| WireError::Truncated)?;
                    if count > cursor.remaining() / MESSAGE_BYTES {
                        return Err(WireError::Truncated);
                    }
                    let mut messages: Vec<Message> = Vec::with_capacity(count);
                    for _ in 0..count {
                        messages.push(cursor.read_message()?);
                    }
                    NetworkResult::Message(Some(messages))
                },
            },
            WireOp::Push => NetworkResult::Push(cursor.read_message()?),
            WireOp::Pull => {
                let msg: Message = cursor.read_message()?;
                let data: Vec<u8> = cursor.read_bytes()?;
                NetworkResult::Pull { msg, data }
            },
        };
        cursor.finish()?;
        Ok(Self { tid, result })
    }

    ///
    /// # Description
    ///
    /// Serializes this response into a complete, length-prefixed wire frame ready to be written to
    /// the transport.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`NetworkResponse::encode`].
    ///
    pub fn to_frame(&self) -> Result<Vec<u8>, WireError> {
        frame_body(&self.encode()?)
    }
}

//==================================================================================================
// Encoding helpers
//==================================================================================================

///
/// # Description
///
/// Wraps an encoded frame body in a length-prefixed wire frame (a little-endian `u32` body length
/// followed by the body bytes).
///
fn frame_body(body: &[u8]) -> Result<Vec<u8>, WireError> {
    let len: u32 = body_len_to_u32(body.len())?;
    let mut frame: Vec<u8> = Vec::with_capacity(LENGTH_PREFIX_BYTES + body.len());
    write_frame_body_len(&mut frame, len);
    frame.extend_from_slice(body);
    Ok(frame)
}

fn write_op(out: &mut Vec<u8>, op: WireOp) {
    out.push(op.to_u8());
}

fn write_presence(out: &mut Vec<u8>, presence: Presence) {
    out.push(presence.to_u8());
}

fn write_tid(out: &mut Vec<u8>, tid: ThreadIdentifier) {
    out.extend_from_slice(&i32::from(tid).to_le_bytes());
}

fn write_frame_body_len(out: &mut Vec<u8>, len: u32) {
    out.extend_from_slice(&len.to_le_bytes());
}

fn write_message_count(out: &mut Vec<u8>, count: u32) {
    out.extend_from_slice(&count.to_le_bytes());
}

fn write_payload_len(out: &mut Vec<u8>, len: u32) {
    out.extend_from_slice(&len.to_le_bytes());
}

fn write_message(out: &mut Vec<u8>, msg: &Message) {
    out.extend_from_slice(&msg.clone().to_bytes());
}

///
/// # Description
///
/// Appends a length-prefixed byte slice to `out` (`u32` little-endian length followed by the
/// bytes).
///
fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), WireError> {
    let len: u32 = payload_len_to_u32(bytes.len())?;
    write_payload_len(out, len);
    out.extend_from_slice(bytes);
    Ok(())
}

fn ensure_frame_len(len: usize) -> Result<(), WireError> {
    if len > MAX_FRAME_BYTES {
        Err(WireError::FrameTooLarge(len))
    } else {
        Ok(())
    }
}

fn checked_frame_len(parts: &[usize]) -> Result<usize, WireError> {
    let mut len: usize = 0;
    for part in parts {
        len = len
            .checked_add(*part)
            .ok_or(WireError::FrameTooLarge(usize::MAX))?;
    }
    ensure_frame_len(len)?;
    Ok(len)
}

fn body_len_to_u32(len: usize) -> Result<u32, WireError> {
    ensure_frame_len(len)?;
    u32::try_from(len).map_err(|_| WireError::FrameTooLarge(len))
}

fn ensure_payload_len(len: usize) -> Result<(), WireError> {
    if len > MAX_PAYLOAD_BYTES {
        Err(WireError::PayloadTooLarge(len))
    } else {
        Ok(())
    }
}

fn payload_len_to_u32(len: usize) -> Result<u32, WireError> {
    ensure_payload_len(len)?;
    u32::try_from(len).map_err(|_| WireError::PayloadTooLarge(len))
}

fn messages_bytes_len(count: usize) -> Result<usize, WireError> {
    count
        .checked_mul(MESSAGE_BYTES)
        .ok_or(WireError::FrameTooLarge(usize::MAX))
}

fn message_count_to_u32(count: usize) -> Result<u32, WireError> {
    let len: usize = messages_bytes_len(count)?;
    ensure_frame_len(len)?;
    u32::try_from(count).map_err(|_| WireError::FrameTooLarge(len))
}

///
/// # Description
///
/// A minimal forward-only cursor over a byte slice used by the wire decoders.
///
struct Cursor<'a> {
    /// The backing byte slice.
    bytes: &'a [u8],
    /// The current read offset.
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], WireError> {
        let end: usize = self.offset.checked_add(len).ok_or(WireError::Truncated)?;
        let slice: &[u8] = self
            .bytes
            .get(self.offset..end)
            .ok_or(WireError::Truncated)?;
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> Result<u8, WireError> {
        Ok(self.take(1)?[0])
    }

    fn read_op(&mut self) -> Result<WireOp, WireError> {
        WireOp::try_from(self.read_u8()?)
    }

    fn read_presence(&mut self) -> Result<Presence, WireError> {
        Presence::try_from(self.read_u8()?)
    }

    fn read_u32(&mut self) -> Result<u32, WireError> {
        let slice: &[u8] = self.take(4)?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_message_count(&mut self) -> Result<u32, WireError> {
        self.read_u32()
    }

    fn read_payload_len(&mut self) -> Result<usize, WireError> {
        let len: usize = usize::try_from(self.read_u32()?).map_err(|_| WireError::Truncated)?;
        ensure_payload_len(len)?;
        Ok(len)
    }

    fn read_i32(&mut self) -> Result<i32, WireError> {
        let slice: &[u8] = self.take(4)?;
        Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_tid(&mut self) -> Result<ThreadIdentifier, WireError> {
        Ok(ThreadIdentifier::from(self.read_i32()?))
    }

    fn read_message(&mut self) -> Result<Message, WireError> {
        let slice: &[u8] = self.take(MESSAGE_BYTES)?;
        let buf: [u8; MESSAGE_BYTES] =
            <[u8; MESSAGE_BYTES]>::try_from(slice).map_err(|_| WireError::InvalidMessage)?;
        Message::try_from_bytes(buf).map_err(|_| WireError::InvalidMessage)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, WireError> {
        let len: usize = self.read_payload_len()?;
        Ok(self.take(len)?.to_vec())
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn finish(&self) -> Result<(), WireError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(WireError::TrailingBytes)
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::sys::{
        error::ErrorCode,
        ipc::{
            MessageReceiver,
            MessageSender,
            MessageType,
        },
        pm::ThreadIdentifier,
    };

    fn sample_message(seed: u8) -> Message {
        let mut payload: [u8; Message::PAYLOAD_SIZE] = [0; Message::PAYLOAD_SIZE];
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte = seed.wrapping_add(i as u8);
        }
        Message::new(
            MessageSender::NETWORKD,
            MessageReceiver::KERNEL,
            MessageType::Ikc,
            None,
            payload,
        )
    }

    fn sample_error_message() -> Message {
        Message::new(
            MessageSender::NETWORKD,
            MessageReceiver::KERNEL,
            MessageType::Ikc,
            Some(ErrorCode::InvalidArgument),
            [0; Message::PAYLOAD_SIZE],
        )
    }

    fn message_bytes(msg: &Message) -> [u8; MESSAGE_BYTES] {
        msg.clone().to_bytes()
    }

    #[test]
    fn request_message_roundtrip() {
        let req = NetworkRequest {
            op: NetworkOp::Message(sample_message(1)),
        };
        let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
        match (decoded.op, req.op) {
            (NetworkOp::Message(a), NetworkOp::Message(b)) => {
                assert_eq!(message_bytes(&a), message_bytes(&b));
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn request_push_roundtrip() {
        let data: Vec<u8> = (0..4096u32).map(|v| v as u8).collect();
        let req = NetworkRequest {
            op: NetworkOp::Push {
                msg: sample_message(7),
                data: data.clone(),
            },
        };
        let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
        match decoded.op {
            NetworkOp::Push {
                msg,
                data: decoded_data,
            } => {
                assert_eq!(message_bytes(&msg), message_bytes(&sample_message(7)));
                assert_eq!(decoded_data, data);
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn request_pull_roundtrip() {
        let req = NetworkRequest {
            op: NetworkOp::Pull(sample_message(9)),
        };
        let decoded = NetworkRequest::decode(&req.encode().unwrap()).unwrap();
        assert!(matches!(decoded.op, NetworkOp::Pull(_)));
    }

    #[test]
    fn response_message_none_roundtrip() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(5),
            result: NetworkResult::Message(None),
        };
        let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
        assert_eq!(decoded.tid, ThreadIdentifier::from(5));
        assert!(matches!(decoded.result, NetworkResult::Message(None)));
    }

    #[test]
    fn response_message_some_roundtrip() {
        let messages = vec![sample_message(1), sample_message(2), sample_message(3)];
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(5),
            result: NetworkResult::Message(Some(messages.clone())),
        };
        let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
        assert_eq!(decoded.tid, ThreadIdentifier::from(5));
        match decoded.result {
            NetworkResult::Message(Some(decoded_messages)) => {
                assert_eq!(decoded_messages.len(), messages.len());
                for (a, b) in decoded_messages.iter().zip(messages.iter()) {
                    assert_eq!(message_bytes(a), message_bytes(b));
                }
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn response_push_roundtrip() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(42),
            result: NetworkResult::Push(sample_message(6)),
        };
        let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
        assert_eq!(decoded.tid, ThreadIdentifier::from(42));
        match decoded.result {
            NetworkResult::Push(msg) => {
                assert_eq!(message_bytes(&msg), message_bytes(&sample_message(6)));
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn response_pull_roundtrip() {
        let data: Vec<u8> = vec![0xAB; 1500];
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(77),
            result: NetworkResult::Pull {
                msg: sample_message(4),
                data: data.clone(),
            },
        };
        let decoded = NetworkResponse::decode(&resp.encode().unwrap()).unwrap();
        assert_eq!(decoded.tid, ThreadIdentifier::from(77));
        match decoded.result {
            NetworkResult::Pull {
                msg,
                data: decoded_data,
            } => {
                assert_eq!(message_bytes(&msg), message_bytes(&sample_message(4)));
                assert_eq!(decoded_data, data);
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn response_bulk_roundtrip_supports_errors() {
        let push_msg: Message = sample_error_message();
        let expected_push_msg: [u8; MESSAGE_BYTES] = message_bytes(&push_msg);
        let push_resp = NetworkResponse {
            tid: ThreadIdentifier::from(42),
            result: NetworkResult::Push(push_msg),
        };
        let decoded: NetworkResponse =
            NetworkResponse::decode(&push_resp.encode().unwrap()).unwrap();
        match decoded.result {
            NetworkResult::Push(msg) => assert_eq!(message_bytes(&msg), expected_push_msg),
            _ => panic!("variant mismatch"),
        }

        let pull_msg: Message = sample_error_message();
        let expected_pull_msg: [u8; MESSAGE_BYTES] = message_bytes(&pull_msg);
        let pull_resp = NetworkResponse {
            tid: ThreadIdentifier::from(77),
            result: NetworkResult::Pull {
                msg: pull_msg,
                data: Vec::new(),
            },
        };
        let decoded: NetworkResponse =
            NetworkResponse::decode(&pull_resp.encode().unwrap()).unwrap();
        match decoded.result {
            NetworkResult::Pull { msg, data } => {
                assert_eq!(message_bytes(&msg), expected_pull_msg);
                assert!(data.is_empty());
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn response_pull_max_payload_reaches_max_frame_size() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(77),
            result: NetworkResult::Pull {
                msg: sample_message(4),
                data: vec![0xAB; MAX_PAYLOAD_BYTES],
            },
        };

        let body: Vec<u8> = resp.encode().unwrap();
        assert_eq!(body.len(), MAX_FRAME_BYTES);
    }

    #[test]
    fn decode_rejects_truncated() {
        assert_eq!(NetworkRequest::decode(&[]).unwrap_err(), WireError::Truncated);
        assert_eq!(
            NetworkRequest::decode(&[WireOp::Message.to_u8()]).unwrap_err(),
            WireError::Truncated
        );
    }

    #[test]
    fn decode_rejects_invalid_op() {
        assert_eq!(NetworkRequest::decode(&[0xFF]).unwrap_err(), WireError::InvalidOp(0xFF));
    }

    #[test]
    fn decode_rejects_invalid_embedded_message() {
        let mut msg: [u8; MESSAGE_BYTES] = sample_message(1).to_bytes();
        msg[0] = 0xFF;

        let mut body: Vec<u8> = Vec::new();
        body.push(WireOp::Message.to_u8());
        body.extend_from_slice(&msg);

        assert_eq!(NetworkRequest::decode(&body).unwrap_err(), WireError::InvalidMessage);
    }

    #[test]
    fn decode_rejects_trailing_bytes() {
        let mut body: Vec<u8> = NetworkRequest {
            op: NetworkOp::Pull(sample_message(1)),
        }
        .encode()
        .unwrap();
        body.push(0);
        assert_eq!(NetworkRequest::decode(&body).unwrap_err(), WireError::TrailingBytes);
    }

    #[test]
    fn request_decode_rejects_oversized_body() {
        let body: Vec<u8> = vec![0; MAX_FRAME_BYTES + 1];
        assert_eq!(
            NetworkRequest::decode(&body).unwrap_err(),
            WireError::FrameTooLarge(MAX_FRAME_BYTES + 1)
        );
    }

    #[test]
    fn response_decode_rejects_oversized_body() {
        let body: Vec<u8> = vec![0; MAX_FRAME_BYTES + 1];
        assert_eq!(
            NetworkResponse::decode(&body).unwrap_err(),
            WireError::FrameTooLarge(MAX_FRAME_BYTES + 1)
        );
    }

    #[test]
    fn encode_rejects_oversized_payload() {
        let req = NetworkRequest {
            op: NetworkOp::Push {
                msg: sample_message(1),
                data: vec![0; MAX_PAYLOAD_BYTES + 1],
            },
        };

        assert_eq!(req.encode().unwrap_err(), WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1));
    }

    #[test]
    fn request_decode_rejects_oversized_payload_before_allocating() {
        let mut body: Vec<u8> = Vec::new();
        body.push(WireOp::Push.to_u8());
        body.extend_from_slice(&sample_message(1).to_bytes());
        body.extend_from_slice(&(u32::try_from(MAX_PAYLOAD_BYTES).unwrap() + 1).to_le_bytes());

        assert_eq!(
            NetworkRequest::decode(&body).unwrap_err(),
            WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1)
        );
    }

    #[test]
    fn response_encode_rejects_oversized_payload() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(5),
            result: NetworkResult::Pull {
                msg: sample_message(1),
                data: vec![0; MAX_PAYLOAD_BYTES + 1],
            },
        };

        assert_eq!(resp.encode().unwrap_err(), WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1));
    }

    #[test]
    fn response_decode_rejects_oversized_payload_before_allocating() {
        let mut body: Vec<u8> = Vec::new();
        body.push(WireOp::Pull.to_u8());
        body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
        body.extend_from_slice(&sample_message(1).to_bytes());
        body.extend_from_slice(&(u32::try_from(MAX_PAYLOAD_BYTES).unwrap() + 1).to_le_bytes());

        assert_eq!(
            NetworkResponse::decode(&body).unwrap_err(),
            WireError::PayloadTooLarge(MAX_PAYLOAD_BYTES + 1)
        );
    }

    #[test]
    fn message_count_error_reports_byte_size() {
        let count: usize = MAX_FRAME_BYTES / MESSAGE_BYTES + 1;
        let len: usize = count * MESSAGE_BYTES;
        assert_eq!(message_count_to_u32(count).unwrap_err(), WireError::FrameTooLarge(len));
    }

    #[test]
    fn response_decode_rejects_impossible_message_count_before_allocating() {
        let mut body: Vec<u8> = Vec::new();
        body.push(WireOp::Message.to_u8());
        body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
        body.push(Presence::Some.to_u8());
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(NetworkResponse::decode(&body).unwrap_err(), WireError::Truncated);
    }

    #[test]
    fn response_decode_rejects_invalid_presence_flag() {
        let mut body: Vec<u8> = Vec::new();
        body.push(WireOp::Message.to_u8());
        body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
        body.push(2);
        assert_eq!(NetworkResponse::decode(&body).unwrap_err(), WireError::InvalidPresence(2));
    }

    #[test]
    fn response_decode_rejects_invalid_op_in_one_byte_body() {
        assert_eq!(NetworkResponse::decode(&[0xFF]).unwrap_err(), WireError::InvalidOp(0xFF));
    }

    #[test]
    fn request_to_frame_is_length_prefixed() {
        let req = NetworkRequest {
            op: NetworkOp::Message(sample_message(1)),
        };
        let body: Vec<u8> = req.encode().unwrap();
        let frame: Vec<u8> = req.to_frame().unwrap();
        assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
        let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(usize::try_from(len).unwrap(), body.len());
        assert_eq!(&frame[LENGTH_PREFIX_BYTES..], body.as_slice());
        // The framed body must decode back to an equivalent request.
        let decoded = NetworkRequest::decode(&frame[LENGTH_PREFIX_BYTES..]).unwrap();
        assert!(matches!(decoded.op, NetworkOp::Message(_)));
    }

    #[test]
    fn response_to_frame_is_length_prefixed() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(11),
            result: NetworkResult::Message(None),
        };
        let body: Vec<u8> = resp.encode().unwrap();
        let frame: Vec<u8> = resp.to_frame().unwrap();
        assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
        let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(usize::try_from(len).unwrap(), body.len());
        assert_eq!(&frame[LENGTH_PREFIX_BYTES..], body.as_slice());
    }
}
