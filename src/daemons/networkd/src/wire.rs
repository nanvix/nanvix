// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt;
use ::sys::{
    ipc::Message,
    pm::ThreadIdentifier,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Size, in bytes, of a serialized [`Message`] on the wire.
pub const MESSAGE_BYTES: usize = Message::HEADER_SIZE + Message::PAYLOAD_SIZE;

/// Number of bytes in the little-endian `u32` length prefix that precedes every wire frame body.
pub const LENGTH_PREFIX_BYTES: usize = 4;

/// Maximum size, in bytes, of a wire frame body.
///
/// Frames larger than this are rejected as a protocol violation so that a corrupt or hostile peer
/// cannot force an unbounded allocation. Datagram payloads are page-bounded, so a 1 MiB ceiling
/// leaves ample headroom for the largest legitimate `sendto`/`recvfrom` frame.
pub const MAX_FRAME_BYTES: usize = 1 << 20;

/// Operation discriminator: inline networking message (`send`, `connect`, ...).
const OP_MESSAGE: u8 = 1;
/// Operation discriminator: `sendto` with a bulk payload.
const OP_SENDTO: u8 = 2;
/// Operation discriminator: `recvfrom` returning a bulk payload.
const OP_RECVFROM: u8 = 3;

/// Presence flag used to encode `Option<Vec<Message>>`: value absent (`None`).
const PRESENCE_NONE: u8 = 0;
/// Presence flag used to encode `Option<Vec<Message>>`: value present (`Some`).
const PRESENCE_SOME: u8 = 1;

//==================================================================================================
// WireError
//==================================================================================================

///
/// # Description
///
/// Error produced while decoding a networkd wire frame.
///
#[derive(Debug, PartialEq, Eq)]
pub enum WireError {
    /// The frame ended before all expected fields were read.
    Truncated,
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
            WireError::InvalidOp(op) => write!(f, "invalid wire op discriminator: {op}"),
            WireError::InvalidPresence(p) => write!(f, "invalid wire presence flag: {p}"),
            WireError::InvalidMessage => write!(f, "invalid embedded message bytes"),
            WireError::TrailingBytes => write!(f, "trailing bytes after wire frame"),
        }
    }
}

impl ::std::error::Error for WireError {}

//==================================================================================================
// NetworkRequest
//==================================================================================================

///
/// # Description
///
/// A request forwarded from the user VM to a decoupled `networkd` process.
///
/// Every request carries the original [`Message`] (never a guest-local memory address) plus, for
/// `sendto`, the already-drained bulk payload. The server reconstructs the `source` and decoded
/// system-call message from the embedded [`Message`] exactly as the in-process handler does, so no
/// additional metadata needs to cross the wire. The originating guest thread identifier — used to
/// correlate the eventual response — is already carried inside the embedded [`Message`], so requests
/// need no explicit correlation field.
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
    /// An inline networking message (e.g. `send`, `connect`, `socket`).
    Message(Message),
    /// A `sendto` request whose bulk payload has been drained from the transport.
    SendTo {
        /// The original request message.
        msg: Message,
        /// The payload bytes to send.
        data: Vec<u8>,
    },
    /// A `recvfrom` request.
    RecvFrom(Message),
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
            NetworkOp::Message(msg) | NetworkOp::RecvFrom(msg) => msg.source.tid,
            NetworkOp::SendTo { msg, .. } => msg.source.tid,
        }
    }
}

impl NetworkRequest {
    ///
    /// # Description
    ///
    /// Serializes this request into the body of a wire frame (without the length prefix).
    ///
    pub fn encode(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        match &self.op {
            NetworkOp::Message(msg) => {
                out.push(OP_MESSAGE);
                out.extend_from_slice(&msg.clone().to_bytes());
            },
            NetworkOp::SendTo { msg, data } => {
                out.push(OP_SENDTO);
                out.extend_from_slice(&msg.clone().to_bytes());
                write_bytes(&mut out, data);
            },
            NetworkOp::RecvFrom(msg) => {
                out.push(OP_RECVFROM);
                out.extend_from_slice(&msg.clone().to_bytes());
            },
        }
        out
    }

    ///
    /// # Description
    ///
    /// Decodes a request from the body of a wire frame (without the length prefix).
    ///
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut cursor: Cursor<'_> = Cursor::new(bytes);
        let op_byte: u8 = cursor.read_u8()?;
        let op: NetworkOp = match op_byte {
            OP_MESSAGE => NetworkOp::Message(cursor.read_message()?),
            OP_SENDTO => {
                let msg: Message = cursor.read_message()?;
                let data: Vec<u8> = cursor.read_bytes()?;
                NetworkOp::SendTo { msg, data }
            },
            OP_RECVFROM => NetworkOp::RecvFrom(cursor.read_message()?),
            other => return Err(WireError::InvalidOp(other)),
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
    pub fn to_frame(&self) -> Vec<u8> {
        frame_body(&self.encode())
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
    /// Result of a [`NetworkOp::SendTo`]: a single response message.
    SendTo(Message),
    /// Result of a [`NetworkOp::RecvFrom`]: a response message plus the received payload.
    RecvFrom {
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
    pub fn encode(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        match &self.result {
            NetworkResult::Message(messages) => {
                out.push(OP_MESSAGE);
                out.extend_from_slice(&i32::from(self.tid).to_le_bytes());
                match messages {
                    None => out.push(PRESENCE_NONE),
                    Some(messages) => {
                        out.push(PRESENCE_SOME);
                        out.extend_from_slice(&(messages.len() as u32).to_le_bytes());
                        for msg in messages {
                            out.extend_from_slice(&msg.clone().to_bytes());
                        }
                    },
                }
            },
            NetworkResult::SendTo(msg) => {
                out.push(OP_SENDTO);
                out.extend_from_slice(&i32::from(self.tid).to_le_bytes());
                out.extend_from_slice(&msg.clone().to_bytes());
            },
            NetworkResult::RecvFrom { msg, data } => {
                out.push(OP_RECVFROM);
                out.extend_from_slice(&i32::from(self.tid).to_le_bytes());
                out.extend_from_slice(&msg.clone().to_bytes());
                write_bytes(&mut out, data);
            },
        }
        out
    }

    ///
    /// # Description
    ///
    /// Decodes a response from the body of a wire frame (without the length prefix).
    ///
    pub fn decode(bytes: &[u8]) -> Result<Self, WireError> {
        let mut cursor: Cursor<'_> = Cursor::new(bytes);
        let op_byte: u8 = cursor.read_u8()?;
        let tid: ThreadIdentifier = ThreadIdentifier::from(cursor.read_i32()?);
        let result: NetworkResult = match op_byte {
            OP_MESSAGE => {
                let presence: u8 = cursor.read_u8()?;
                match presence {
                    PRESENCE_NONE => NetworkResult::Message(None),
                    PRESENCE_SOME => {
                        let count: u32 = cursor.read_u32()?;
                        let count: usize =
                            usize::try_from(count).map_err(|_| WireError::Truncated)?;
                        if count > cursor.remaining() / MESSAGE_BYTES {
                            return Err(WireError::Truncated);
                        }
                        let mut messages: Vec<Message> = Vec::with_capacity(count);
                        for _ in 0..count {
                            messages.push(cursor.read_message()?);
                        }
                        NetworkResult::Message(Some(messages))
                    },
                    other => return Err(WireError::InvalidPresence(other)),
                }
            },
            OP_SENDTO => NetworkResult::SendTo(cursor.read_message()?),
            OP_RECVFROM => {
                let msg: Message = cursor.read_message()?;
                let data: Vec<u8> = cursor.read_bytes()?;
                NetworkResult::RecvFrom { msg, data }
            },
            other => return Err(WireError::InvalidOp(other)),
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
    pub fn to_frame(&self) -> Vec<u8> {
        frame_body(&self.encode())
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
fn frame_body(body: &[u8]) -> Vec<u8> {
    let mut frame: Vec<u8> = Vec::with_capacity(LENGTH_PREFIX_BYTES + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
    frame.extend_from_slice(body);
    frame
}

///
/// # Description
///
/// Appends a length-prefixed byte slice to `out` (`u32` little-endian length followed by the
/// bytes).
///
fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
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

    fn read_u32(&mut self) -> Result<u32, WireError> {
        let slice: &[u8] = self.take(4)?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_i32(&mut self) -> Result<i32, WireError> {
        let slice: &[u8] = self.take(4)?;
        Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn read_message(&mut self) -> Result<Message, WireError> {
        let slice: &[u8] = self.take(MESSAGE_BYTES)?;
        let buf: [u8; MESSAGE_BYTES] =
            <[u8; MESSAGE_BYTES]>::try_from(slice).map_err(|_| WireError::InvalidMessage)?;
        Message::try_from_bytes(buf).map_err(|_| WireError::InvalidMessage)
    }

    fn read_bytes(&mut self) -> Result<Vec<u8>, WireError> {
        let len: u32 = self.read_u32()?;
        Ok(self.take(len as usize)?.to_vec())
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

    fn message_bytes(msg: &Message) -> [u8; MESSAGE_BYTES] {
        msg.clone().to_bytes()
    }

    #[test]
    fn request_message_roundtrip() {
        let req = NetworkRequest {
            op: NetworkOp::Message(sample_message(1)),
        };
        let decoded = NetworkRequest::decode(&req.encode()).unwrap();
        match (decoded.op, req.op) {
            (NetworkOp::Message(a), NetworkOp::Message(b)) => {
                assert_eq!(message_bytes(&a), message_bytes(&b));
            },
            _ => panic!("variant mismatch"),
        }
    }

    #[test]
    fn request_sendto_roundtrip() {
        let data: Vec<u8> = (0..4096u32).map(|v| v as u8).collect();
        let req = NetworkRequest {
            op: NetworkOp::SendTo {
                msg: sample_message(7),
                data: data.clone(),
            },
        };
        let decoded = NetworkRequest::decode(&req.encode()).unwrap();
        match decoded.op {
            NetworkOp::SendTo {
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
    fn request_recvfrom_roundtrip() {
        let req = NetworkRequest {
            op: NetworkOp::RecvFrom(sample_message(9)),
        };
        let decoded = NetworkRequest::decode(&req.encode()).unwrap();
        assert!(matches!(decoded.op, NetworkOp::RecvFrom(_)));
    }

    #[test]
    fn response_message_none_roundtrip() {
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(5),
            result: NetworkResult::Message(None),
        };
        let decoded = NetworkResponse::decode(&resp.encode()).unwrap();
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
        let decoded = NetworkResponse::decode(&resp.encode()).unwrap();
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
    fn response_recvfrom_roundtrip() {
        let data: Vec<u8> = vec![0xAB; 1500];
        let resp = NetworkResponse {
            tid: ThreadIdentifier::from(77),
            result: NetworkResult::RecvFrom {
                msg: sample_message(4),
                data: data.clone(),
            },
        };
        let decoded = NetworkResponse::decode(&resp.encode()).unwrap();
        assert_eq!(decoded.tid, ThreadIdentifier::from(77));
        match decoded.result {
            NetworkResult::RecvFrom {
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
    fn decode_rejects_truncated() {
        assert_eq!(NetworkRequest::decode(&[]).unwrap_err(), WireError::Truncated);
        assert_eq!(NetworkRequest::decode(&[OP_MESSAGE]).unwrap_err(), WireError::Truncated);
    }

    #[test]
    fn decode_rejects_invalid_op() {
        assert_eq!(NetworkRequest::decode(&[0xFF]).unwrap_err(), WireError::InvalidOp(0xFF));
    }

    #[test]
    fn decode_rejects_trailing_bytes() {
        let mut body: Vec<u8> = NetworkRequest {
            op: NetworkOp::RecvFrom(sample_message(1)),
        }
        .encode();
        body.push(0);
        assert_eq!(NetworkRequest::decode(&body).unwrap_err(), WireError::TrailingBytes);
    }

    #[test]
    fn response_decode_rejects_impossible_message_count_before_allocating() {
        let mut body: Vec<u8> = Vec::new();
        body.push(OP_MESSAGE);
        body.extend_from_slice(&i32::from(ThreadIdentifier::from(5)).to_le_bytes());
        body.push(PRESENCE_SOME);
        body.extend_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(NetworkResponse::decode(&body).unwrap_err(), WireError::Truncated);
    }

    #[test]
    fn request_to_frame_is_length_prefixed() {
        let req = NetworkRequest {
            op: NetworkOp::Message(sample_message(1)),
        };
        let body: Vec<u8> = req.encode();
        let frame: Vec<u8> = req.to_frame();
        assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
        let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(len as usize, body.len());
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
        let body: Vec<u8> = resp.encode();
        let frame: Vec<u8> = resp.to_frame();
        assert_eq!(frame.len(), LENGTH_PREFIX_BYTES + body.len());
        let len: u32 = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
        assert_eq!(len as usize, body.len());
        assert_eq!(&frame[LENGTH_PREFIX_BYTES..], body.as_slice());
    }
}
