// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::{
    io::{
        Error,
        ErrorKind,
        Read,
        Write,
    },
    mem,
    os::unix::net::UnixStream,
};
use ::sys::ipc::Message;

//==================================================================================================
// Structure
//==================================================================================================

pub enum Gateway {
    UnixStream(UnixStream),
}

impl Gateway {
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
    pub fn try_send(&mut self, message: Message) -> Result<(), Error> {
        match self {
            Gateway::UnixStream(stream) => {
                let bytes: [u8; mem::size_of::<Message>()] = message.to_bytes();
                match stream.write_all(&bytes) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                        if e.kind() != ErrorKind::WouldBlock {
                            let reason: String = format!("failed to send message ({:?}", e);
                            error!("send(): {}", reason);
                        }

                        Err(e)
                    },
                }
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
    pub fn try_receive(&mut self) -> Result<Message, Error> {
        match self {
            Gateway::UnixStream(stream) => {
                let mut bytes: [u8; mem::size_of::<Message>()] = [0; mem::size_of::<Message>()];
                match stream.read_exact(&mut bytes) {
                    Ok(_) => {
                        let message: Message = match Message::try_from_bytes(bytes) {
                            Ok(message) => message,
                            Err(e) => {
                                let reason: String = format!("failed to parse message ({:?})", e);
                                error!("receive(): {}", reason);
                                return Err(Error::new(ErrorKind::InvalidData, reason));
                            },
                        };

                        Ok(message)
                    },
                    Err(e) => {
                        // Print error messages only if it is not a WouldBlock error to avoid spamming the logs.
                        if e.kind() != ErrorKind::WouldBlock {
                            let reason: String = format!("failed to receive message ({:?})", e);
                            error!("receive(): {}", reason);
                        }

                        Err(e)
                    },
                }
            },
        }
    }
}
