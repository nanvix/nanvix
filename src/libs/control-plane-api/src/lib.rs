// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! Control Plane API
//!
//! This library provides a structured wire protocol for control-plane messages exchanged between
//! different components of the Nanvix system, including the Nanvix Daemon (nanvixd), Linux Daemon
//! (linuxd), and User VMs.
//!

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Imports
//==================================================================================================

use ::num_enum::{
    IntoPrimitive,
    TryFromPrimitive,
};
use ::std::{
    io::{
        Error,
        ErrorKind,
    },
    mem,
};
use ::syslog::error;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Command
///
#[derive(Debug, Clone, Copy, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum NanvixdCommand {
    /// Shutdown.
    Shutdown,
}

///
/// # Description
///
/// Control message sent by Nanvix Daemon (nanvixd).
///
pub struct NanvixdControlMessage {
    /// Command.
    command: NanvixdCommand,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NanvixdControlMessage {
    ///
    /// # Description
    ///
    /// Creates a new control message.
    ///
    /// # Parameters
    ///
    /// - `command`: Command to be sent.
    ///
    /// # Returns
    ///
    /// The newly created control message.
    ///
    pub fn new(command: NanvixdCommand) -> Self {
        Self { command }
    }

    ///
    /// # Description
    ///
    /// Returns the command of this control message.
    ///
    /// # Returns
    ///
    /// The command of this control message.
    ///
    pub fn cmd(&self) -> NanvixdCommand {
        self.command
    }

    ///
    /// # Description
    ///
    /// Serializes the command into a byte array.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Buffer to serialize the command into.
    ///
    pub fn to_bytes(&self, buffer: &mut [u8; mem::size_of::<Self>()]) {
        let command_bytes: u8 = self.command.into();
        buffer[0] = command_bytes;
    }

    ///
    /// # Description
    ///
    /// Tries to deserialize a command from a byte array.
    ///
    /// # Parameters
    ///
    /// - `buffer`: Buffer to deserialize the command from.
    ///
    /// # Returns
    ///
    /// On success, this function returns the deserialized command. On failure, it returns an error.
    ///
    pub fn try_from_bytes(buffer: &[u8; mem::size_of::<Self>()]) -> Result<Self, Error> {
        let command = NanvixdCommand::try_from(buffer[0]).map_err(|_| {
            let reason: String = format!("invalid command: {}", buffer[0]);
            error!("try_from_bytes(): {reason}");
            Error::new(ErrorKind::InvalidData, reason)
        })?;
        Ok(Self { command })
    }
}
