// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! User VM API
//!
//! This library provides the communication protocol between User VMs and the Linux Daemon (linuxd).
//! It defines messages exchanged during VM registration and the establishment of communication
//! channels.
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
use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::{
    fmt,
    io,
    io::Result,
};
use ::syscomm::{
    SocketAddr,
    SocketType,
};

//==================================================================================================
// Types
//==================================================================================================

///
/// # Description
///
/// Unique identifier for each user VM.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UserVmIdentifier {
    /// Underlying numeric identifier.
    value: u32,
}

/// Message sent by a UserVM to linuxd to register itself right after booting.
#[derive(Clone, Debug)]
pub struct NewUserVm {
    /// Unique identifier for the user VM.
    user_vm_id: UserVmIdentifier,
    /// Socket address that users can read/write to communicate with the VM's stdin/stdout.
    gateway_sockaddr: String,
    /// Type of gateway socket to connect to.
    gateway_socket_type: SocketType,
    /// Direct-ring transport information for this user VM.
    ring_transport: RingTransport,
}

/// Kind of direct-ring transport attached to a user VM registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RingTransportKind {
    /// No direct shared-ring transport.
    Disabled = 0,
    /// Shared ring backed by a host file path.
    FilePath = 1,
    /// Shared ring backed by an ivshmem locator.
    Ivshmem = 2,
}

/// Transport information for the direct shared-ring path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingTransport {
    /// Transport kind.
    kind: RingTransportKind,
    /// Transport locator interpreted according to `kind`.
    locator: String,
}

//==================================================================================================
// Constants
//==================================================================================================

const GATEWAY_SOCKADDR_MAX_LEN: usize =
    if SocketAddr::UNIX_SOCKADDR_MAX_LEN >= SocketAddr::TCP_SOCKADDR_MAX_LEN {
        SocketAddr::UNIX_SOCKADDR_MAX_LEN
    } else {
        SocketAddr::TCP_SOCKADDR_MAX_LEN
    };

const USER_VM_IDENTIFIER_LEN: usize = ::std::mem::size_of::<u32>();
const SOCKET_TYPE_LEN: usize = ::std::mem::size_of::<u8>();
const RING_TRANSPORT_KIND_LEN: usize = ::std::mem::size_of::<u8>();
const SOCKET_TYPE_OFFSET: usize = USER_VM_IDENTIFIER_LEN;
const RING_TRANSPORT_KIND_OFFSET: usize = SOCKET_TYPE_OFFSET + SOCKET_TYPE_LEN;
const RING_TRANSPORT_LOCATOR_MAX_LEN: usize = 256;

const NEW_USER_VM_HEADER_LEN: usize =
    USER_VM_IDENTIFIER_LEN + SOCKET_TYPE_LEN + RING_TRANSPORT_KIND_LEN;

const GATEWAY_SOCKADDR_OFFSET: usize = NEW_USER_VM_HEADER_LEN;
const RING_TRANSPORT_LOCATOR_OFFSET: usize = GATEWAY_SOCKADDR_OFFSET + GATEWAY_SOCKADDR_MAX_LEN;

pub const NEW_USER_VM_MESSAGE_LEN: usize =
    NEW_USER_VM_HEADER_LEN + GATEWAY_SOCKADDR_MAX_LEN + RING_TRANSPORT_LOCATOR_MAX_LEN;

/// One-byte control frame sent by uservm to linuxd to report new SQ work on a direct ring.
pub const DIRECT_RING_SQ_DOORBELL_FRAME: u8 = 0x80;

/// One-byte control frame sent by linuxd to uservm to request a guest CQ notification.
pub const DIRECT_RING_CQ_DOORBELL_FRAME: u8 = 0x81;

/// Auto-discovery locator for an ivshmem-backed direct ring.
pub const IVSHMEM_RING_TRANSPORT_LOCATOR_AUTO: &str = "auto";

//==================================================================================================
// Structures
//==================================================================================================

impl From<UserVmIdentifier> for u32 {
    fn from(id: UserVmIdentifier) -> Self {
        id.value
    }
}

impl fmt::Display for UserVmIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl UserVmIdentifier {
    ///
    /// # Description
    ///
    /// Creates a new `RawUserVmIdentifier` from a raw `u32` value.
    ///
    /// # Parameters
    ///
    /// - `value`: Raw value of the identifier.
    ///
    /// # Return Value
    ///
    /// The newly created identifier.
    ///
    pub const fn new(value: u32) -> Self {
        Self { value }
    }
}

impl TryFrom<u8> for RingTransportKind {
    type Error = io::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Disabled),
            1 => Ok(Self::FilePath),
            2 => Ok(Self::Ivshmem),
            _ => {
                let reason: &str = "invalid value for ring_transport_kind";
                error!("RingTransportKind::try_from(): {reason} (value={value})");
                Err(io::Error::new(io::ErrorKind::InvalidInput, reason))
            },
        }
    }
}

impl From<RingTransportKind> for u8 {
    fn from(kind: RingTransportKind) -> Self {
        kind as u8
    }
}

impl RingTransport {
    fn new(kind: RingTransportKind, locator: String) -> Result<Self> {
        if locator.len() > RING_TRANSPORT_LOCATOR_MAX_LEN {
            let reason: String = format!(
                "ring transport locator too long (max: {}, got: {})",
                RING_TRANSPORT_LOCATOR_MAX_LEN,
                locator.len()
            );
            error!("RingTransport::new(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
        }

        if kind == RingTransportKind::Disabled && !locator.is_empty() {
            let reason: &str = "disabled ring transport cannot carry a locator";
            error!("RingTransport::new(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
        }

        Ok(Self { kind, locator })
    }

    /// Creates a disabled transport descriptor.
    pub fn disabled() -> Self {
        Self {
            kind: RingTransportKind::Disabled,
            locator: String::new(),
        }
    }

    /// Creates a file-backed transport descriptor.
    pub fn file_path(path: String) -> Result<Self> {
        Self::new(RingTransportKind::FilePath, path)
    }

    /// Creates an ivshmem-backed transport descriptor.
    pub fn ivshmem(locator: String) -> Result<Self> {
        Self::new(RingTransportKind::Ivshmem, locator)
    }

    /// Returns the transport kind.
    pub fn kind(&self) -> RingTransportKind {
        self.kind
    }

    /// Returns the transport locator.
    pub fn locator(&self) -> &str {
        self.locator.as_ref()
    }
}

impl NewUserVm {
    ///
    /// # Description
    ///
    /// Creates a new `NewUserVm` message.
    ///
    /// # Parameters
    ///
    /// - `user_vm_id`: Unique identifier for the user VM.
    /// - `gateway_sockaddr`: Socket address that users can read/write to communicate with the VM.
    /// - `gateway_socket_type`: Type of gateway socket to connect to.
    ///
    /// # Return Value
    ///
    /// The newly created `NewUserVm` message.
    ///
    pub fn new(
        user_vm_id: UserVmIdentifier,
        gateway_sockaddr: String,
        gateway_socket_type: SocketType,
        ring_transport: RingTransport,
    ) -> Result<Self> {
        // Check if the socket address length is invalid.
        let sockaddr_len: usize = gateway_sockaddr.len();
        if (gateway_socket_type == SocketType::Unix)
            && (sockaddr_len > SocketAddr::UNIX_SOCKADDR_MAX_LEN)
        {
            let reason: String = format!(
                "unix socket address too long (max: {}, got: {})",
                SocketAddr::UNIX_SOCKADDR_MAX_LEN,
                sockaddr_len
            );
            error!("NewUserVm::new(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
        } else if (gateway_socket_type == SocketType::Tcp)
            && (sockaddr_len > SocketAddr::TCP_SOCKADDR_MAX_LEN)
        {
            let reason: String = format!(
                "tcp socket address too long (max: {}, got: {})",
                SocketAddr::TCP_SOCKADDR_MAX_LEN,
                sockaddr_len
            );
            error!("NewUserVm::new(): {reason}");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
        }

        Ok(Self {
            user_vm_id,
            gateway_sockaddr,
            gateway_socket_type,
            ring_transport,
        })
    }

    ///
    /// # Description
    ///
    /// Get the identifier of the user VM.
    ///
    /// # Return Value
    ///
    /// The identifier of the user VM.
    ///
    pub fn id(&self) -> UserVmIdentifier {
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
    pub fn gateway_socket_type(&self) -> &SocketType {
        &self.gateway_socket_type
    }

    pub fn ring_transport(&self) -> &RingTransport {
        &self.ring_transport
    }

    pub fn to_bytes(&self) -> [u8; NEW_USER_VM_MESSAGE_LEN] {
        let mut encoded: [u8; NEW_USER_VM_MESSAGE_LEN] = [0u8; NEW_USER_VM_MESSAGE_LEN];

        let id_bytes: [u8; USER_VM_IDENTIFIER_LEN] = self.user_vm_id.value.to_le_bytes();
        encoded[..USER_VM_IDENTIFIER_LEN].copy_from_slice(&id_bytes);

        encoded[SOCKET_TYPE_OFFSET] = self.gateway_socket_type.into();
        encoded[RING_TRANSPORT_KIND_OFFSET] = self.ring_transport.kind.into();

        let sockaddr_bytes: &[u8] = self.gateway_sockaddr.as_bytes();
        encoded[GATEWAY_SOCKADDR_OFFSET..GATEWAY_SOCKADDR_OFFSET + sockaddr_bytes.len()]
            .copy_from_slice(sockaddr_bytes);

        let ring_transport_locator_bytes: &[u8] = self.ring_transport.locator.as_bytes();
        encoded[RING_TRANSPORT_LOCATOR_OFFSET
            ..RING_TRANSPORT_LOCATOR_OFFSET + ring_transport_locator_bytes.len()]
            .copy_from_slice(ring_transport_locator_bytes);

        encoded
    }

    pub fn try_from_bytes(bytes: &[u8; NEW_USER_VM_MESSAGE_LEN]) -> Result<Self> {
        let mut encoded: [u8; NEW_USER_VM_MESSAGE_LEN] = [0u8; NEW_USER_VM_MESSAGE_LEN];
        encoded.copy_from_slice(&bytes[..NEW_USER_VM_MESSAGE_LEN]);

        let mut id_bytes: [u8; USER_VM_IDENTIFIER_LEN] = [0u8; USER_VM_IDENTIFIER_LEN];
        id_bytes.copy_from_slice(&encoded[..USER_VM_IDENTIFIER_LEN]);
        let user_vm_id: u32 = u32::from_le_bytes(id_bytes);

        let gateway_socket_type_byte: u8 = encoded[SOCKET_TYPE_OFFSET];
        let gateway_socket_type: SocketType = SocketType::try_from(gateway_socket_type_byte)
            .map_err(|_| {
                let reason: &str = "invalid value for gateway_socket_type";
                error!("NewUserVm::try_from_bytes(): {reason}");
                io::Error::new(io::ErrorKind::InvalidInput, reason)
            })?;

        let ring_transport_kind: RingTransportKind =
            RingTransportKind::try_from(encoded[RING_TRANSPORT_KIND_OFFSET])?;

        let sockaddr_start: usize = GATEWAY_SOCKADDR_OFFSET;
        let sockaddr_end: usize = sockaddr_start + GATEWAY_SOCKADDR_MAX_LEN;
        let raw_sockaddr: &[u8] = &encoded[sockaddr_start..sockaddr_end];

        let sockaddr_len: usize = raw_sockaddr
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(GATEWAY_SOCKADDR_MAX_LEN);
        let gateway_sockaddr: String = String::from_utf8(raw_sockaddr[..sockaddr_len].to_vec())
            .map_err(|_| {
                let reason: &str = "invalid UTF-8 in gateway_sockaddr";
                error!("NewUserVm::try_from_bytes(): {reason}");
                io::Error::new(io::ErrorKind::InvalidInput, reason)
            })?;

        let locator_start: usize = RING_TRANSPORT_LOCATOR_OFFSET;
        let locator_end: usize = locator_start + RING_TRANSPORT_LOCATOR_MAX_LEN;
        let raw_locator: &[u8] = &encoded[locator_start..locator_end];

        let locator_len: usize = raw_locator
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(raw_locator.len());
        let ring_transport_locator: String = String::from_utf8(raw_locator[..locator_len].to_vec())
            .map_err(|_| {
                let reason: &str = "invalid UTF-8 in ring_transport_locator";
                error!("NewUserVm::try_from_bytes(): {reason}");
                io::Error::new(io::ErrorKind::InvalidInput, reason)
            })?;
        let ring_transport: RingTransport =
            RingTransport::new(ring_transport_kind, ring_transport_locator)?;

        Self::new(
            UserVmIdentifier::new(user_vm_id),
            gateway_sockaddr,
            gateway_socket_type,
            ring_transport,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NewUserVm,
        RingTransport,
        RingTransportKind,
        UserVmIdentifier,
        NEW_USER_VM_MESSAGE_LEN,
    };
    use ::std::io::Result;
    use ::syscomm::SocketType;

    #[test]
    fn new_user_vm_round_trips_disabled_ring_transport() -> Result<()> {
        let message: NewUserVm = NewUserVm::new(
            UserVmIdentifier::new(7),
            "127.0.0.1:1234".to_string(),
            SocketType::Tcp,
            RingTransport::disabled(),
        )?;

        let encoded: [u8; NEW_USER_VM_MESSAGE_LEN] = message.to_bytes();
        let decoded: NewUserVm = NewUserVm::try_from_bytes(&encoded)?;

        assert_eq!(decoded.id(), UserVmIdentifier::new(7));
        assert_eq!(decoded.gateway_sockaddr(), "127.0.0.1:1234");
        assert_eq!(decoded.ring_transport().kind(), RingTransportKind::Disabled);
        assert_eq!(decoded.ring_transport().locator(), "");
        Ok(())
    }

    #[test]
    fn new_user_vm_round_trips_ivshmem_ring_transport() -> Result<()> {
        let message: NewUserVm = NewUserVm::new(
            UserVmIdentifier::new(11),
            "gateway.sock".to_string(),
            SocketType::Unix,
            RingTransport::ivshmem("device=nanvix-ring0".to_string())?,
        )?;

        let encoded: [u8; NEW_USER_VM_MESSAGE_LEN] = message.to_bytes();
        let decoded: NewUserVm = NewUserVm::try_from_bytes(&encoded)?;

        assert_eq!(decoded.ring_transport().kind(), RingTransportKind::Ivshmem);
        assert_eq!(decoded.ring_transport().locator(), "device=nanvix-ring0");
        Ok(())
    }
}
