// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! This file contains the messages used in the control-plane API between the control-plane
//! (nanvixd), the system VM (linuxd), and the user VM. It defines a simple wire-format to allow
//! for bi-directional communication. The specification for the wire-format is as follows:
//! - source: u8 -> Source of the control-plane message (Nanvixd, SystemVm, or UserVm).
//! - length: u32 LE -> Length of the message payload.
//! - bytes: [u8; length]
//!

#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]

use ::anyhow::Result;
use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::io::{
    Error,
    ErrorKind,
};
use ::syscomm::{
    SocketError,
    SocketStream,
};
use ::syslog::error;

///
/// # Description
///
/// This enum indicates which entity is the source of a given control-plane message.
///
#[repr(u8)]
#[derive(Clone, Copy, Debug, IntoPrimitive, PartialEq, TryFromPrimitive)]
pub enum Source {
    Nanvixd,
    SystemVm,
    UserVm,
}

///
/// # Description
///
/// Wire format for all control-plane messages, irrespective of the source.
///
pub trait WireCommand: Serialize + for<'de> Deserialize<'de> {
    const SOURCE: Source;
}

///
/// # Description
///
/// Default bincode options to encode/decode control-plane messages. This configuration uses
/// fixed-size ints, and little-endian encoding.
///
const fn bincode_cfg() -> impl bincode::config::Config {
    bincode::config::standard()
        .with_fixed_int_encoding()
        .with_little_endian()
}

///
/// # Description
///
/// Control-plane messages from Nanvixd to the system VM and the user VM. For the moment we do not
/// differentiate between who is the recipient.
///
#[derive(Debug, Serialize, Deserialize)]
pub enum NanvixdCommand {
    Shutdown,
}

impl WireCommand for NanvixdCommand {
    const SOURCE: Source = Source::Nanvixd;
}

///
/// # Description
///
/// Send a message following the control-plane protocol.
///
/// # Arguments
///
/// - `stream`: the control-plane stream where to send the message.
/// - `msg`: a message that implements the WireCommand trait.
///
/// # Returns
///
/// In case of success, returns nothing. Otherwise, returns an error.
///
pub fn send_command<T: WireCommand>(stream: &mut SocketStream, msg: &T) -> Result<(), SocketError> {
    let payload: Vec<u8> = bincode::serde::encode_to_vec(msg, bincode_cfg()).map_err(|e| {
        let reason: String = format!("failed to encode control-plane command (error={e:?})");
        error!("{reason}");
        Error::new(ErrorKind::InvalidData, reason)
    })?;

    if payload.len() > u32::MAX as usize {
        let reason: String = format!(
            "payload length exceeds maximum allowed size (length={}, max={})",
            payload.len(),
            u32::MAX
        );
        error!("{reason}");
        return Err(Error::new(ErrorKind::InvalidData, reason).into());
    }
    let len: [u8; 4] = u32::try_from(payload.len())
        .map_err(|_| {
            let reason: String =
                format!("error parsing payload length to u32 (len={})", payload.len());
            error!("{reason}");
            Error::new(ErrorKind::InvalidData, reason)
        })?
        .to_le_bytes();

    // Send source, then payload length, then payload.
    stream.write_all(&[T::SOURCE as u8])?;
    stream.write_all(&len)?;
    if !payload.is_empty() {
        stream.write_all(&payload)?;
    }

    Ok(())
}

///
/// # Description
///
/// Receive a generic control-plane message following the wire-format.
///
/// # Arguments
///
/// - `stream`: the control-plane stream where to send the message.
///
/// # Returns
///
/// In case of success, a message of the indicated trait. Otherwise, an error.
///
pub fn recv_command<T: WireCommand>(stream: &mut SocketStream) -> Result<T, SocketError> {
    // Read command source.
    let mut source_bytes: [u8; 1] = [0u8; 1];
    let num_read: usize = stream.try_read_exact(&mut source_bytes)?;
    debug_assert_eq!(num_read, 1);

    let source: Source = Source::try_from(source_bytes[0]).map_err(|_| {
        let reason: String =
            format!("error parsing source in control-plane command (bytes={source_bytes:?})");
        error!("{reason}");
        SocketError::from(Error::new(ErrorKind::InvalidData, reason))
    })?;
    if source != T::SOURCE {
        let reason: String = format!(
            "unexpected control-plane command source (got={}, expected={})",
            source as u8,
            T::SOURCE as u8
        );
        error!("{reason}");
        return Err(Error::new(ErrorKind::InvalidData, reason).into());
    }

    // Read command length.
    let mut length_bytes: [u8; 4] = [0u8; 4];
    let num_read: usize = stream.try_read_exact(&mut length_bytes)?;
    debug_assert_eq!(num_read, 4);
    let len: usize = u32::from_le_bytes(length_bytes) as usize;

    // Sanity-check the received length.
    if len > config::syscomm::MAX_MESSAGE_LEN {
        let reason: String = format!("received control-plane message too large (len={len})");
        error!("{reason}");
        return Err(Error::new(ErrorKind::InvalidData, reason).into());
    }

    // Read payload.
    let mut message_buf: Vec<u8> = vec![0u8; len];
    if len > 0 {
        let num_read: usize = stream.try_read_exact(&mut message_buf)?;
        debug_assert_eq!(num_read, len);
    }

    // Decode message.
    let (msg, _): (T, usize) = bincode::serde::decode_from_slice(&message_buf, bincode_cfg())
        .map_err(|e| {
            let reason: String = format!("failed to decode message (error={e:?})");
            error!("{reason}");
            SocketError::from(Error::new(ErrorKind::InvalidData, reason))
        })?;

    Ok(msg)
}
