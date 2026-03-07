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

use ::log::error;
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

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Command sent by Nanvix Daemon (nanvixd) to Linux Daemon (linuxd).
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

///
/// # Description
///
/// Command sent by Linux Daemon (linuxd) to Nanvix Daemon (nanvixd).
///
#[derive(Debug, Clone, Copy, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[repr(u8)]
pub enum LinuxdCommand {
    /// Signals that the gateway listener has been bound and is ready to accept connections.
    GatewayReady,
}

///
/// # Description
///
/// Control message sent by Linux Daemon (linuxd).
///
pub struct LinuxdControlMessage {
    /// Command.
    command: LinuxdCommand,
    /// Identifier of the User VM that this message pertains to.
    gateway_id: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NanvixdControlMessage {
    /// Wire size of the serialized message: 1 byte command.
    pub const WIRE_SIZE: usize = 1;

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
    pub fn to_bytes(&self, buffer: &mut [u8; Self::WIRE_SIZE]) {
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
    pub fn try_from_bytes(buffer: &[u8; Self::WIRE_SIZE]) -> Result<Self, Error> {
        let command = NanvixdCommand::try_from(buffer[0]).map_err(|_| {
            let reason: String = format!("invalid command: {}", buffer[0]);
            error!("try_from_bytes(): {reason}");
            Error::new(ErrorKind::InvalidData, reason)
        })?;
        Ok(Self { command })
    }
}

impl LinuxdControlMessage {
    /// Wire size of the serialized message: 1 byte command + 4 bytes gateway_id.
    pub const WIRE_SIZE: usize = 1 + mem::size_of::<u32>();

    ///
    /// # Description
    ///
    /// Creates a new control message.
    ///
    /// # Parameters
    ///
    /// - `command`: Command to be sent.
    /// - `gateway_id`: Identifier of the User VM that this message pertains to.
    ///
    /// # Returns
    ///
    /// The newly created control message.
    ///
    pub fn new(command: LinuxdCommand, gateway_id: u32) -> Self {
        Self {
            command,
            gateway_id,
        }
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
    pub fn cmd(&self) -> LinuxdCommand {
        self.command
    }

    ///
    /// # Description
    ///
    /// Returns the gateway identifier of this control message.
    ///
    /// # Returns
    ///
    /// The User VM identifier that this message pertains to.
    ///
    pub fn gateway_id(&self) -> u32 {
        self.gateway_id
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
    pub fn to_bytes(&self, buffer: &mut [u8; Self::WIRE_SIZE]) {
        let command_bytes: u8 = self.command.into();
        buffer[0] = command_bytes;
        buffer[1..5].copy_from_slice(&self.gateway_id.to_le_bytes());
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
    pub fn try_from_bytes(buffer: &[u8; Self::WIRE_SIZE]) -> Result<Self, Error> {
        let command = LinuxdCommand::try_from(buffer[0]).map_err(|_| {
            let reason: String = format!("invalid linuxd command: {}", buffer[0]);
            error!("try_from_bytes(): {reason}");
            Error::new(ErrorKind::InvalidData, reason)
        })?;
        let gateway_id: u32 = u32::from_le_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]);
        Ok(Self {
            command,
            gateway_id,
        })
    }
}
