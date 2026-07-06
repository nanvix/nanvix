// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wire::{
    LENGTH_PREFIX_BYTES,
    MAX_FRAME_BYTES,
};
use ::std::io::{
    Error,
    ErrorKind,
    Result,
};
use ::syscomm::{
    ReadExact,
    SocketStreamReader,
};

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads a single length-prefixed frame body from `reader`.
///
/// Every frame on the wire is a [`LENGTH_PREFIX_BYTES`]-byte little-endian length followed by that
/// many body bytes. This is the single shared reader used by both the `networkd` server and the
/// user-VM client, so the two ends can never drift in how they frame requests and responses.
///
/// # Cancellation
///
/// This reader is only cancel-correct when awaited as the sole future in its task, as both the
/// server reader loop and the client reader task do. It must not be dropped mid-frame inside a
/// `select!`; the user VM's I/O thread deliberately keeps its own incremental, cancel-safe reader
/// for that reason.
///
/// # Parameters
///
/// - `reader`: The read half of the connection.
///
/// # Returns
///
/// - `Ok(Some(body))` with the frame body on success.
/// - `Ok(None)` if the peer disconnected cleanly at a frame boundary.
/// - `Err(_)` on an oversized frame or an unexpected transport error.
///
pub async fn read_frame(reader: &mut SocketStreamReader) -> Result<Option<Vec<u8>>> {
    let mut len_buf: [u8; LENGTH_PREFIX_BYTES] = [0; LENGTH_PREFIX_BYTES];

    // A clean disconnect surfaces as an unexpected EOF while reading the length prefix.
    if let Err(e) = reader.read_exact(&mut len_buf).await {
        if e.kind() == ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e);
    }

    let len: usize = usize::try_from(u32::from_le_bytes(len_buf))
        .map_err(|_| Error::new(ErrorKind::InvalidData, "networkd frame length overflow"))?;
    if len > MAX_FRAME_BYTES {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("networkd frame too large: {len} bytes (max {MAX_FRAME_BYTES})"),
        ));
    }

    let mut body: Vec<u8> = vec![0; len];
    reader.read_exact(&mut body).await?;
    Ok(Some(body))
}
