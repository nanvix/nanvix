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
use ::user_vm_api::UserVmIdentifier;

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

///
/// # Description
///
/// Kind of peer registering on the shared control-plane listener.
///
#[derive(Debug, Clone, Copy, IntoPrimitive, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum ControlPlanePeerKind {
    /// Linux daemon instance.
    LinuxDaemon,
    /// User VM instance.
    UserVm,
}

///
/// # Description
///
/// Registration message sent immediately after a peer connects to the control-plane listener.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlPlaneRegistrationMessage {
    peer_kind: ControlPlanePeerKind,
    user_vm_id: u32,
    tenant_id: String,
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
        let command: NanvixdCommand = NanvixdCommand::try_from(buffer[0]).map_err(|_| {
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
        let command: LinuxdCommand = LinuxdCommand::try_from(buffer[0]).map_err(|_| {
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

impl ControlPlaneRegistrationMessage {
    /// Wire header size: 1 byte peer kind + 4 bytes user VM id + 2 bytes tenant-id length.
    pub const HEADER_SIZE: usize = 1 + mem::size_of::<u32>() + mem::size_of::<u16>();

    /// Byte offset of the peer-kind field within the wire header.
    pub const PEER_KIND_OFFSET: usize = 0;

    /// Byte offset of the user-VM-id field within the wire header.
    pub const USER_VM_ID_OFFSET: usize = Self::PEER_KIND_OFFSET + 1;

    /// Byte offset of the tenant-id-length field within the wire header.
    pub const TENANT_ID_LEN_OFFSET: usize = Self::USER_VM_ID_OFFSET + mem::size_of::<u32>();

    ///
    /// # Description
    ///
    /// Creates a registration message for a Linux daemon connection.
    ///
    /// # Arguments
    ///
    /// - `tenant_id`: Tenant identifier associated with the Linux daemon instance. Must not be
    ///   empty.
    ///
    /// # Returns
    ///
    /// Returns the newly created registration message if `tenant_id` is valid.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] if `tenant_id` is empty.
    ///
    pub fn for_linuxd(tenant_id: &str) -> Result<Self, Error> {
        if tenant_id.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tenant_id cannot be empty for Linux daemon registration",
            ));
        }

        Ok(Self {
            peer_kind: ControlPlanePeerKind::LinuxDaemon,
            user_vm_id: 0,
            tenant_id: tenant_id.to_string(),
        })
    }

    ///
    /// # Description
    ///
    /// Creates a registration message for a User VM connection.
    ///
    /// # Arguments
    ///
    /// - `user_vm_id`: Identifier of the User VM instance.
    ///
    /// # Returns
    ///
    /// Returns the newly created registration message.
    ///
    pub fn for_uservm(user_vm_id: UserVmIdentifier) -> Self {
        Self {
            peer_kind: ControlPlanePeerKind::UserVm,
            user_vm_id: user_vm_id.into(),
            tenant_id: String::new(),
        }
    }

    ///
    /// # Description
    ///
    /// Returns the peer kind encoded in this message.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the kind of the peer that emitted this registration.
    ///
    pub fn peer_kind(&self) -> ControlPlanePeerKind {
        self.peer_kind
    }

    ///
    /// # Description
    ///
    /// Returns the User VM identifier if this is a User VM registration.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the registered User VM identifier when present.
    ///
    pub fn user_vm_id(&self) -> Option<UserVmIdentifier> {
        if self.peer_kind == ControlPlanePeerKind::UserVm {
            Some(UserVmIdentifier::new(self.user_vm_id))
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Returns the tenant identifier if this is a Linux daemon registration.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the registered tenant identifier when present.
    ///
    pub fn tenant_id(&self) -> Option<&str> {
        if self.peer_kind == ControlPlanePeerKind::LinuxDaemon {
            Some(&self.tenant_id)
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Returns the exact serialized size of this message.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the serialized size in bytes.
    ///
    pub fn wire_size(&self) -> usize {
        Self::HEADER_SIZE + self.tenant_id.len()
    }

    ///
    /// # Description
    ///
    /// Serializes this registration message.
    ///
    /// # Arguments
    ///
    /// This function takes no arguments.
    ///
    /// # Returns
    ///
    /// Returns the serialized registration message bytes.
    ///
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let tenant_id_bytes: &[u8] = self.tenant_id.as_bytes();
        let tenant_id_len: u16 = tenant_id_bytes.len().try_into().map_err(|_| {
            let reason: String = format!("tenant identifier too long: {}", tenant_id_bytes.len());
            error!("to_bytes(): {reason}");
            Error::new(ErrorKind::InvalidInput, reason)
        })?;
        let mut bytes: Vec<u8> = vec![0u8; self.wire_size()];
        bytes[Self::PEER_KIND_OFFSET] = self.peer_kind.into();
        bytes[Self::USER_VM_ID_OFFSET..Self::TENANT_ID_LEN_OFFSET]
            .copy_from_slice(&self.user_vm_id.to_le_bytes());
        bytes[Self::TENANT_ID_LEN_OFFSET..Self::HEADER_SIZE]
            .copy_from_slice(&tenant_id_len.to_le_bytes());
        bytes[Self::HEADER_SIZE..].copy_from_slice(tenant_id_bytes);
        Ok(bytes)
    }

    ///
    /// # Description
    ///
    /// Deserializes a registration message from its header and tenant-id payload.
    ///
    /// # Arguments
    ///
    /// - `header`: Fixed-size registration header.
    /// - `tenant_id_bytes`: Variable-size tenant identifier payload.
    ///
    /// # Returns
    ///
    /// Returns the decoded registration message.
    ///
    pub fn try_from_parts(
        header: &[u8; Self::HEADER_SIZE],
        tenant_id_bytes: &[u8],
    ) -> Result<Self, Error> {
        let peer_kind: ControlPlanePeerKind =
            ControlPlanePeerKind::try_from(header[Self::PEER_KIND_OFFSET]).map_err(|_| {
                let reason: String =
                    format!("invalid control-plane peer kind: {}", header[Self::PEER_KIND_OFFSET]);
                error!("try_from_parts(): {reason}");
                Error::new(ErrorKind::InvalidData, reason)
            })?;
        let user_vm_id: u32 = u32::from_le_bytes([
            header[Self::USER_VM_ID_OFFSET],
            header[Self::USER_VM_ID_OFFSET + 1],
            header[Self::USER_VM_ID_OFFSET + 2],
            header[Self::USER_VM_ID_OFFSET + 3],
        ]);
        let tenant_id_len: usize = usize::from(u16::from_le_bytes([
            header[Self::TENANT_ID_LEN_OFFSET],
            header[Self::TENANT_ID_LEN_OFFSET + 1],
        ]));
        if tenant_id_len != tenant_id_bytes.len() {
            let reason: String = format!(
                "tenant identifier length mismatch (header={tenant_id_len}, payload={})",
                tenant_id_bytes.len()
            );
            error!("try_from_parts(): {reason}");
            return Err(Error::new(ErrorKind::InvalidData, reason));
        }
        let tenant_id: String = String::from_utf8(tenant_id_bytes.to_vec()).map_err(|error| {
            let reason: String = format!("invalid tenant identifier encoding (error={error:?})");
            error!("try_from_parts(): {reason}");
            Error::new(ErrorKind::InvalidData, reason)
        })?;

        match peer_kind {
            ControlPlanePeerKind::LinuxDaemon if tenant_id.is_empty() => {
                let reason: &str = "linux daemon registration missing tenant identifier";
                error!("try_from_parts(): {reason}");
                Err(Error::new(ErrorKind::InvalidData, reason))
            },
            ControlPlanePeerKind::LinuxDaemon => Ok(Self {
                peer_kind,
                user_vm_id: 0,
                tenant_id,
            }),
            ControlPlanePeerKind::UserVm if !tenant_id.is_empty() => {
                let reason: &str = "user VM registration should not include tenant identifier";
                error!("try_from_parts(): {reason}");
                Err(Error::new(ErrorKind::InvalidData, reason))
            },
            ControlPlanePeerKind::UserVm => Ok(Self {
                peer_kind,
                user_vm_id,
                tenant_id,
            }),
        }
    }
}
