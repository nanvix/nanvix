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

    /// Maximum allowed length (in bytes) for a tenant identifier. Tenant identifiers are short
    /// human-readable strings (e.g., UUIDs, project slugs). A 256-byte ceiling is well above any
    /// realistic identifier while still preventing a malicious peer from triggering large allocations
    /// through the 2-byte (`u16`) wire-length field.
    pub const MAX_TENANT_ID_LEN: usize = 256;

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
    /// Returns [`ErrorKind::InvalidInput`] if `tenant_id` is empty or exceeds
    /// [`Self::MAX_TENANT_ID_LEN`] bytes.
    ///
    pub fn for_linuxd(tenant_id: &str) -> Result<Self, Error> {
        if tenant_id.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "tenant_id cannot be empty for Linux daemon registration",
            ));
        }
        if tenant_id.len() > Self::MAX_TENANT_ID_LEN {
            let reason: String = format!(
                "tenant_id length {} exceeds maximum {}",
                tenant_id.len(),
                Self::MAX_TENANT_ID_LEN
            );
            error!("for_linuxd(): {reason}");
            return Err(Error::new(ErrorKind::InvalidInput, reason));
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
        if tenant_id_len > Self::MAX_TENANT_ID_LEN {
            let reason: String = format!(
                "tenant_id length {tenant_id_len} exceeds maximum {}",
                Self::MAX_TENANT_ID_LEN
            );
            error!("try_from_parts(): {reason}");
            return Err(Error::new(ErrorKind::InvalidData, reason));
        }
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

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ::serde::{
        Deserialize,
        Serialize,
    };

    /// Mirrors the `Kill` message struct from `nanvix-http::message` so that the test suite can
    /// verify JSON wire-format compatibility without pulling in the full HTTP crate.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct Kill {
        user_vm_id: UserVmIdentifier,
    }

    // ==================== UserVmIdentifier JSON Tests ====================

    /// `UserVmIdentifier` must serialize as a nested object `{"value": N}`, not a bare integer.
    #[test]
    fn user_vm_identifier_serializes_as_nested_object() {
        let id = UserVmIdentifier::new(42);
        let result = serde_json::to_string(&id);
        assert!(result.is_ok(), "serialize UserVmIdentifier should succeed");
        if let Ok(json) = result {
            assert_eq!(
                json, r#"{"value":42}"#,
                "UserVmIdentifier should serialize as nested object"
            );
        }
    }

    /// A bare integer must fail to deserialize as `UserVmIdentifier`.
    #[test]
    fn user_vm_identifier_rejects_bare_integer() {
        let result = serde_json::from_str::<UserVmIdentifier>("42");
        assert!(result.is_err(), "bare integer should not deserialize as UserVmIdentifier");
    }

    /// The nested `{{"value": N}}` format must round-trip through serde.
    #[test]
    fn user_vm_identifier_roundtrip() {
        let id = UserVmIdentifier::new(4132086170);
        let result = serde_json::to_string(&id);
        assert!(result.is_ok(), "serialize UserVmIdentifier should succeed");
        if let Ok(json) = result {
            let result2 = serde_json::from_str::<UserVmIdentifier>(&json);
            assert!(result2.is_ok(), "deserialize UserVmIdentifier should succeed");
            if let Ok(recovered) = result2 {
                assert_eq!(id, recovered, "UserVmIdentifier should round-trip through JSON");
            }
        }
    }

    /// A Kill message with `{"user_vm_id": N}` (bare integer) must fail deserialization — this is
    /// the exact bug from issue #2017.
    #[test]
    fn kill_message_rejects_bare_integer_user_vm_id() {
        let bad_json = r#"{"user_vm_id":4132086170}"#;
        let result = serde_json::from_str::<Kill>(bad_json);
        assert!(result.is_err(), "Kill with bare-integer user_vm_id should fail deserialization");
    }

    /// A Kill message with `{"user_vm_id": {"value": N}}` must deserialize correctly.
    #[test]
    fn kill_message_accepts_nested_user_vm_id() {
        let good_json = r#"{"user_vm_id":{"value":4132086170}}"#;
        let result = serde_json::from_str::<Kill>(good_json);
        assert!(result.is_ok(), "Kill with nested user_vm_id should succeed");
        if let Ok(msg) = result {
            assert_eq!(
                msg.user_vm_id,
                UserVmIdentifier::new(4132086170),
                "parsed user_vm_id should match"
            );
        }
    }

    /// A Kill message must round-trip through serde_json and always produce the nested format.
    #[test]
    fn kill_message_serialization_roundtrip() {
        let msg = Kill {
            user_vm_id: UserVmIdentifier::new(99),
        };
        let result = serde_json::to_string(&msg);
        assert!(result.is_ok(), "serialize Kill should succeed");
        if let Ok(json) = result {
            assert_eq!(
                json, r#"{"user_vm_id":{"value":99}}"#,
                "Kill should serialize with nested user_vm_id"
            );
            let result2 = serde_json::from_str::<Kill>(&json);
            assert!(result2.is_ok(), "deserialize Kill should succeed");
            if let Ok(recovered) = result2 {
                assert_eq!(
                    recovered.user_vm_id, msg.user_vm_id,
                    "Kill should round-trip through JSON"
                );
            }
        }
    }

    // ==================== NanvixdControlMessage Tests ====================

    /// WIRE_SIZE must equal 1 byte.
    #[test]
    fn nanvixd_control_message_wire_size() {
        assert_eq!(NanvixdControlMessage::WIRE_SIZE, 1, "wire size should be 1 byte");
    }

    /// `new()` followed by `cmd()` must return the same command.
    #[test]
    fn nanvixd_control_message_new_and_cmd() {
        let msg = NanvixdControlMessage::new(NanvixdCommand::Shutdown);
        assert_eq!(msg.cmd(), NanvixdCommand::Shutdown, "cmd() should return Shutdown");
    }

    /// Shutdown command must round-trip through `to_bytes` / `try_from_bytes`.
    #[test]
    fn nanvixd_control_message_roundtrip_shutdown() {
        let msg = NanvixdControlMessage::new(NanvixdCommand::Shutdown);
        let mut buf = [0u8; NanvixdControlMessage::WIRE_SIZE];
        msg.to_bytes(&mut buf);
        let result = NanvixdControlMessage::try_from_bytes(&buf);
        assert!(result.is_ok(), "deserialize Shutdown should succeed");
        if let Ok(recovered) = result {
            assert_eq!(
                recovered.cmd(),
                NanvixdCommand::Shutdown,
                "roundtrip should preserve command"
            );
        }
    }

    /// An invalid command byte must be rejected by `try_from_bytes`.
    #[test]
    fn nanvixd_control_message_rejects_invalid_command() {
        let buf = [0xFFu8; NanvixdControlMessage::WIRE_SIZE];
        let result = NanvixdControlMessage::try_from_bytes(&buf);
        assert!(result.is_err(), "invalid command byte 0xFF should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }

    // ==================== LinuxdControlMessage Tests ====================

    /// WIRE_SIZE must equal 5 bytes (1 command + 4 gateway_id).
    #[test]
    fn linuxd_control_message_wire_size() {
        assert_eq!(LinuxdControlMessage::WIRE_SIZE, 5, "wire size should be 5 bytes");
    }

    /// `new()` followed by accessors must return the construction arguments.
    #[test]
    fn linuxd_control_message_new_and_accessors() {
        let msg = LinuxdControlMessage::new(LinuxdCommand::GatewayReady, 42);
        assert_eq!(msg.cmd(), LinuxdCommand::GatewayReady, "cmd() should return GatewayReady");
        assert_eq!(msg.gateway_id(), 42, "gateway_id() should return 42");
    }

    /// GatewayReady command must round-trip through `to_bytes` / `try_from_bytes`.
    #[test]
    fn linuxd_control_message_roundtrip() {
        let msg = LinuxdControlMessage::new(LinuxdCommand::GatewayReady, 0xDEADBEEF);
        let mut buf = [0u8; LinuxdControlMessage::WIRE_SIZE];
        msg.to_bytes(&mut buf);
        let result = LinuxdControlMessage::try_from_bytes(&buf);
        assert!(result.is_ok(), "deserialize GatewayReady should succeed");
        if let Ok(recovered) = result {
            assert_eq!(
                recovered.cmd(),
                LinuxdCommand::GatewayReady,
                "roundtrip should preserve command"
            );
            assert_eq!(recovered.gateway_id(), 0xDEADBEEF, "roundtrip should preserve gateway_id");
        }
    }

    /// Zero gateway_id round-trips correctly.
    #[test]
    fn linuxd_control_message_roundtrip_zero_gateway_id() {
        let msg = LinuxdControlMessage::new(LinuxdCommand::GatewayReady, 0);
        let mut buf = [0u8; LinuxdControlMessage::WIRE_SIZE];
        msg.to_bytes(&mut buf);
        let result = LinuxdControlMessage::try_from_bytes(&buf);
        assert!(result.is_ok(), "deserialize should succeed");
        if let Ok(recovered) = result {
            assert_eq!(recovered.gateway_id(), 0, "zero gateway_id should round-trip");
        }
    }

    /// u32::MAX gateway_id round-trips correctly.
    #[test]
    fn linuxd_control_message_roundtrip_max_gateway_id() {
        let msg = LinuxdControlMessage::new(LinuxdCommand::GatewayReady, u32::MAX);
        let mut buf = [0u8; LinuxdControlMessage::WIRE_SIZE];
        msg.to_bytes(&mut buf);
        let result = LinuxdControlMessage::try_from_bytes(&buf);
        assert!(result.is_ok(), "deserialize should succeed");
        if let Ok(recovered) = result {
            assert_eq!(recovered.gateway_id(), u32::MAX, "u32::MAX gateway_id should round-trip");
        }
    }

    /// An invalid command byte must be rejected by `try_from_bytes`.
    #[test]
    fn linuxd_control_message_rejects_invalid_command() {
        let buf = [0xFFu8; LinuxdControlMessage::WIRE_SIZE];
        let result = LinuxdControlMessage::try_from_bytes(&buf);
        assert!(result.is_err(), "invalid command byte 0xFF should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }

    // ==================== ControlPlaneRegistrationMessage Tests ====================

    /// HEADER_SIZE must equal 7 bytes (1 peer_kind + 4 user_vm_id + 2 tenant_id_len).
    #[test]
    fn registration_header_size() {
        assert_eq!(ControlPlaneRegistrationMessage::HEADER_SIZE, 7, "header should be 7 bytes");
    }

    /// `for_linuxd` creates a LinuxDaemon registration with the given tenant_id.
    #[test]
    fn registration_for_linuxd_valid() {
        let result = ControlPlaneRegistrationMessage::for_linuxd("tenant-abc");
        assert!(result.is_ok(), "valid tenant_id should be accepted");
        if let Ok(msg) = result {
            assert_eq!(msg.peer_kind(), ControlPlanePeerKind::LinuxDaemon, "should be LinuxDaemon");
            assert_eq!(
                msg.tenant_id(),
                Some("tenant-abc"),
                "tenant_id should match construction argument"
            );
            assert_eq!(msg.user_vm_id(), None, "LinuxDaemon should not have user_vm_id");
        }
    }

    /// `for_linuxd` rejects an empty tenant_id.
    #[test]
    fn registration_for_linuxd_rejects_empty_tenant() {
        let result = ControlPlaneRegistrationMessage::for_linuxd("");
        assert!(result.is_err(), "empty tenant_id should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidInput, "error kind should be InvalidInput");
        }
    }

    /// `for_linuxd` rejects a tenant_id that exceeds MAX_TENANT_ID_LEN.
    #[test]
    fn registration_for_linuxd_rejects_oversized_tenant() {
        let long_tenant = "a".repeat(ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN + 1);
        let result = ControlPlaneRegistrationMessage::for_linuxd(&long_tenant);
        assert!(result.is_err(), "oversized tenant_id should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidInput, "error kind should be InvalidInput");
        }
    }

    /// `for_linuxd` accepts a tenant_id at exactly MAX_TENANT_ID_LEN.
    #[test]
    fn registration_for_linuxd_accepts_max_len_tenant() {
        let max_tenant = "b".repeat(ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN);
        let result = ControlPlaneRegistrationMessage::for_linuxd(&max_tenant);
        assert!(result.is_ok(), "max-length tenant_id should be accepted");
        if let Ok(msg) = result {
            assert_eq!(msg.tenant_id(), Some(max_tenant.as_str()), "tenant_id should match");
        }
    }

    /// `for_uservm` creates a UserVm registration with the given identifier.
    #[test]
    fn registration_for_uservm_valid() {
        let vm_id = UserVmIdentifier::new(777);
        let msg = ControlPlaneRegistrationMessage::for_uservm(vm_id);
        assert_eq!(msg.peer_kind(), ControlPlanePeerKind::UserVm, "should be UserVm");
        assert_eq!(msg.user_vm_id(), Some(vm_id), "user_vm_id should match construction argument");
        assert_eq!(msg.tenant_id(), None, "UserVm should not have tenant_id");
    }

    /// `wire_size` for a linuxd registration includes header + tenant_id bytes.
    #[test]
    fn registration_wire_size_linuxd() {
        let result = ControlPlaneRegistrationMessage::for_linuxd("hello");
        assert!(result.is_ok(), "valid tenant_id should be accepted");
        if let Ok(msg) = result {
            assert_eq!(
                msg.wire_size(),
                ControlPlaneRegistrationMessage::HEADER_SIZE + 5,
                "wire_size should be HEADER_SIZE + tenant_id length"
            );
        }
    }

    /// `wire_size` for a uservm registration is exactly HEADER_SIZE (empty tenant_id).
    #[test]
    fn registration_wire_size_uservm() {
        let msg = ControlPlaneRegistrationMessage::for_uservm(UserVmIdentifier::new(1));
        assert_eq!(
            msg.wire_size(),
            ControlPlaneRegistrationMessage::HEADER_SIZE,
            "wire_size should be HEADER_SIZE for UserVm"
        );
    }

    /// LinuxDaemon registration must round-trip through `to_bytes` / `try_from_parts`.
    #[test]
    fn registration_roundtrip_linuxd() {
        let result = ControlPlaneRegistrationMessage::for_linuxd("my-tenant");
        assert!(result.is_ok(), "valid tenant should be accepted");
        if let Ok(original) = result {
            let result2 = original.to_bytes();
            assert!(result2.is_ok(), "serialize should succeed");
            if let Ok(bytes) = result2 {
                let header_result: Result<&[u8; ControlPlaneRegistrationMessage::HEADER_SIZE], _> =
                    bytes[..ControlPlaneRegistrationMessage::HEADER_SIZE].try_into();
                assert!(header_result.is_ok(), "header slice should succeed");
                if let Ok(header) = header_result {
                    let tenant_payload = &bytes[ControlPlaneRegistrationMessage::HEADER_SIZE..];
                    let result3 =
                        ControlPlaneRegistrationMessage::try_from_parts(header, tenant_payload);
                    assert!(result3.is_ok(), "deserialize should succeed");
                    if let Ok(recovered) = result3 {
                        assert_eq!(recovered, original, "linuxd registration should round-trip");
                    }
                }
            }
        }
    }

    /// UserVm registration must round-trip through `to_bytes` / `try_from_parts`.
    #[test]
    fn registration_roundtrip_uservm() {
        let original =
            ControlPlaneRegistrationMessage::for_uservm(UserVmIdentifier::new(0xCAFEBABE));
        let result = original.to_bytes();
        assert!(result.is_ok(), "serialize should succeed");
        if let Ok(bytes) = result {
            let header_result: Result<&[u8; ControlPlaneRegistrationMessage::HEADER_SIZE], _> =
                bytes[..ControlPlaneRegistrationMessage::HEADER_SIZE].try_into();
            assert!(header_result.is_ok(), "header slice should succeed");
            if let Ok(header) = header_result {
                let tenant_payload = &bytes[ControlPlaneRegistrationMessage::HEADER_SIZE..];
                let result2 =
                    ControlPlaneRegistrationMessage::try_from_parts(header, tenant_payload);
                assert!(result2.is_ok(), "deserialize should succeed");
                if let Ok(recovered) = result2 {
                    assert_eq!(recovered, original, "uservm registration should round-trip");
                }
            }
        }
    }

    /// `try_from_parts` rejects an invalid peer-kind byte.
    #[test]
    fn registration_rejects_invalid_peer_kind() {
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] = 0xFF;
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, &[]);
        assert!(result.is_err(), "invalid peer kind should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }

    /// `try_from_parts` rejects a LinuxDaemon registration with an empty tenant_id payload.
    #[test]
    fn registration_rejects_linuxd_without_tenant() {
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::LinuxDaemon.into();
        // tenant_id_len = 0, payload = empty
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, &[]);
        assert!(result.is_err(), "linuxd without tenant_id should be rejected");
    }

    /// `try_from_parts` rejects a UserVm registration that includes a tenant_id payload.
    #[test]
    fn registration_rejects_uservm_with_tenant() {
        let tenant = b"unexpected";
        // "unexpected" is 10 bytes — use the literal to avoid a truncating cast.
        let tenant_len = 10u16.to_le_bytes();
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::UserVm.into();
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET] = tenant_len[0];
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1] = tenant_len[1];
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, tenant);
        assert!(result.is_err(), "uservm with tenant_id should be rejected");
    }

    /// `try_from_parts` rejects a header whose tenant_id_len does not match the payload length.
    #[test]
    fn registration_rejects_tenant_length_mismatch() {
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::LinuxDaemon.into();
        // Claim 10 bytes in the header but provide only 3 in the payload.
        let tenant_len = 10u16.to_le_bytes();
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET] = tenant_len[0];
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1] = tenant_len[1];
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, b"abc");
        assert!(result.is_err(), "length mismatch should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }

    /// `try_from_parts` rejects a tenant_id_len that exceeds MAX_TENANT_ID_LEN.
    #[test]
    fn registration_rejects_oversized_tenant_in_header() {
        // MAX_TENANT_ID_LEN is 256, so MAX + 1 = 257 = 0x0101.
        let huge_len = 257u16.to_le_bytes();
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::LinuxDaemon.into();
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET] = huge_len[0];
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1] = huge_len[1];
        let payload = vec![b'x'; ControlPlaneRegistrationMessage::MAX_TENANT_ID_LEN + 1];
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, &payload);
        assert!(result.is_err(), "oversized tenant_id_len in header should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }

    /// `try_from_parts` rejects a tenant_id payload that is not valid UTF-8.
    #[test]
    fn registration_rejects_invalid_utf8_tenant() {
        let bad_bytes: &[u8] = &[0xFF, 0xFE];
        // bad_bytes is 2 bytes long.
        let tenant_len = 2u16.to_le_bytes();
        let mut header = [0u8; ControlPlaneRegistrationMessage::HEADER_SIZE];
        header[ControlPlaneRegistrationMessage::PEER_KIND_OFFSET] =
            ControlPlanePeerKind::LinuxDaemon.into();
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET] = tenant_len[0];
        header[ControlPlaneRegistrationMessage::TENANT_ID_LEN_OFFSET + 1] = tenant_len[1];
        let result = ControlPlaneRegistrationMessage::try_from_parts(&header, bad_bytes);
        assert!(result.is_err(), "invalid UTF-8 tenant_id should be rejected");
        if let Err(e) = result {
            assert_eq!(e.kind(), ErrorKind::InvalidData, "error kind should be InvalidData");
        }
    }
}
