// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This module provides a simple API that the user VM can use to communicate with linuxd (in the
//! system VM) right after start-up.
//!
//! We use a simple wire-format where we prefix each message with a u32 containing the message
//! length. We then use bincode to encode/decode the message struct.

//==================================================================================================
// Imports
//==================================================================================================

use ::bincode::{
    config,
    Decode,
    Encode,
};
use ::std::io;
use ::syscomm::{
    BlockingSocketStream,
    SocketType,
};
use ::syslog::error;

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// Unique identifier for each user VM.
///
pub type RawUserVmIdentifier = u32;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This message is sent by the user VM to the system VM right after they have established a
/// connection so that the user VM can identify itself to the system VM.
///
#[derive(Clone, Debug, Decode, Encode)]
pub struct NewUserVm {
    user_vm_id: RawUserVmIdentifier,
    /// Socket address that users can read/write to communicate with the VM's stdin/stdout.
    gateway_sockaddr: String,
    gateway_socket_type: SocketType,
}

impl NewUserVm {
    pub fn new(
        user_vm_id: RawUserVmIdentifier,
        gateway_sockaddr: String,
        gateway_socket_type: SocketType,
    ) -> Self {
        Self {
            user_vm_id,
            gateway_sockaddr,
            gateway_socket_type,
        }
    }

    pub fn id(&self) -> RawUserVmIdentifier {
        self.user_vm_id
    }

    ///
    /// # Description
    ///
    /// Get the gateway socket address.
    ///
    /// # Return Value
    ///
    /// The gateway socket address.
    ///
    pub fn gateway_sockaddr(&self) -> &str {
        self.gateway_sockaddr.as_ref()
    }

    ///
    /// # Description
    ///
    /// Get the gateway socket type.
    ///
    /// # Return Value
    ///
    /// The gateway socket type.
    ///
    pub fn gateway_socket_type(&self) -> SocketType {
        self.gateway_socket_type.clone()
    }

    ///
    /// # Description
    ///
    /// Sends a [`NewUserVm`] message through a blocking stream
    ///
    /// # Parameters
    ///
    /// - `blocking_stream`: Blocking stream to which the message should be sent.
    ///
    /// # Return Value
    ///
    /// On successful completion, this function returns empty. Otherwise, it returns an error
    /// indicating the reason for the failure.
    ///
    pub fn send(&self, blocking_stream: &mut BlockingSocketStream) -> io::Result<()> {
        let payload: Vec<u8> =
            bincode::encode_to_vec(self, config::standard()).map_err(|encode_error| {
                let reason: String =
                    format!("failed to serialize message (error={encode_error:?})");
                error!("send(): {reason}");
                io::Error::new(io::ErrorKind::InvalidData, reason)
            })?;

        // Check if serialized message is too large.
        if payload.is_empty() || payload.len() > ::config::syscomm::MAX_MESSAGE_LEN {
            let reason: String = format!("invalid message length (length={})", payload.len());
            error!("send(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
        }

        let len_be: [u8; size_of::<u32>()] = (payload.len() as u32).to_be_bytes();

        blocking_stream.write_all(&len_be)?;
        blocking_stream.write_all(&payload)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Receive a [`NewUserVm`] message from a blocking stream.
    ///
    /// # Parameters
    ///
    /// - `blocking_stream`: Blocking stream from which the message should be received.
    ///
    /// # Returns
    ///
    /// On successful completion, this function returns the message received from the referred
    /// blocking stream. Otherwise, it returns an error indicating the reason for the failure.
    ///
    pub fn recv(blocking_stream: &mut BlockingSocketStream) -> io::Result<Self> {
        // Read the size of the message.
        let mut len_buf: [u8; size_of::<u32>()] = [0u8; size_of::<u32>()];
        blocking_stream.read_exact(&mut len_buf)?;
        let len: usize = u32::from_be_bytes(len_buf) as usize;

        // Check if message has an invalid size.
        if len == 0 || len > ::config::syscomm::MAX_MESSAGE_LEN {
            let reason: String = format!("invalid message length (length={len})");
            error!("recv(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidData, reason));
        }

        // Read the message body.
        let mut buf: Vec<u8> = vec![0u8; len];
        blocking_stream.read_exact(&mut buf)?;

        let (msg, msg_len): (NewUserVm, usize) =
            bincode::decode_from_slice(&buf, config::standard()).map_err(|decode_error| {
                let reason: String =
                    format!("failed to deserialize message (error={decode_error:?})");
                error!("{reason}");
                io::Error::new(io::ErrorKind::InvalidData, reason)
            })?;
        debug_assert!(msg_len == len);

        Ok(msg)
    }
}
