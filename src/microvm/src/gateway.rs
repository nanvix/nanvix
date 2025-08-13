// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    collections::VecDeque,
    io::{
        Error,
        ErrorKind,
    },
    mem,
};
use ::sys::ipc::Message;
use ::syscomm::{
    SocketError,
    SocketStream,
};

//==================================================================================================
// Structure
//==================================================================================================

pub struct Gateway {
    stream: SocketStream,
    partial_read_buffer: VecDeque<u8>,
}

impl Gateway {
    ///
    /// # Description
    ///
    /// Creates a new gateway.
    ///
    /// # Parameters
    ///
    /// - `stream`: Socket stream to be used by the gateway.
    ///
    /// # Returns
    ///
    /// A new gateway instance.
    ///
    pub fn new(stream: SocketStream) -> Self {
        Gateway {
            stream,
            partial_read_buffer: VecDeque::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to send a message to the gateway.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to be sent.
    ///
    /// # Returns
    ///
    /// If the message was successfully sent, `Ok(())` is returned. Otherwise, an error is returned.
    ///
    pub fn try_send(&mut self, message: Message) -> Result<(), SocketError> {
        let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();
        match self.stream.write_all(&bytes) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                if e.kind() != ErrorKind::WouldBlock {
                    let reason: String = format!("failed to send message ({e:?}");
                    error!("send(): {reason}");
                }

                Err(e)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Attempts to receive a message from the gateway.
    ///
    /// # Returns
    ///
    /// Upon success, the received message is returned. Otherwise, an error is returned.
    ///
    pub fn try_receive(&mut self) -> Result<Message, SocketError> {
        let mut buf: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];

        let mut num_filled = 0;
        if !self.partial_read_buffer.is_empty() {
            // Prepare data in buffer for partial read.
            self.partial_read_buffer.make_contiguous();
            let partial_bytes = self.partial_read_buffer.as_slices().0;

            // We take the minimum at the end just in case, but the partial read should always be
            // strictly smaller than the message size.
            let num_partial_read = partial_bytes.len().min(buf.len());

            buf[..num_partial_read].copy_from_slice(&partial_bytes[..num_partial_read]);

            // Clear partial read buffer.
            self.partial_read_buffer.clear();
            num_filled += num_partial_read;
        }
        // Post-condition: partial_read_buffer is empty.

        match self.stream.try_read_exact(&mut buf[num_filled..]) {
            Ok(n) => {
                // Handle partial reads by copying all we have read to the partial read buffer and
                // returning a WouldBlock indicating that we need more data.
                if n + num_filled < buf.len() {
                    self.partial_read_buffer.extend(&buf[..(n + num_filled)]);
                    return Err(std::io::Error::new(ErrorKind::WouldBlock, "partial read").into());
                }
            },
            Err(e) => {
                // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                if e.kind() != ErrorKind::WouldBlock {
                    let reason: String = format!("failed to receive message ({e:?})");
                    error!("receive(): {reason}");
                }

                return Err(SocketError::new(e.into()));
            },
        };

        let mut message: Message = match Message::try_from_bytes(buf) {
            Ok(message) => message,
            Err(e) => {
                let reason: String = format!("failed to parse message ({e:?})");
                error!("receive(): {reason}");
                return Err(SocketError::new(Error::new(ErrorKind::InvalidData, reason)));
            },
        };
        profiler::timestamp_message!(
            &mut message.payload,
            mem::offset_of!(syscall::LinuxDaemonMessage, payload)
                + mem::offset_of!(syscall::unistd::message::ReadResponse, buffer)
        );
        Ok(message)
    }
}
