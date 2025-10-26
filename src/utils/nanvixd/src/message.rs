// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Messages used in the HTTP API between end-clients and nanvixd.
//!
//! This module defines the message types and structures for communication between external
//! clients and the Nanvix Daemon. It includes messages for creating new sandboxes, killing
//! existing ones, and their corresponding responses.

use ::serde::{
    Deserialize,
    Serialize,
};
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Message to create a new User VM instance managed by nanvixd.
///
/// This message is sent by external clients to request the creation of a new sandboxed
/// execution environment.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct New {
    /// Tenant identifier for resource isolation and multi-tenancy support.
    pub tenant_id: String,
    /// Application name for identification and organization.
    pub app_name: String,
    /// Path to the program binary to execute inside the User VM.
    pub program: String,
    /// Command-line arguments to pass to the program.
    pub program_args: String,
}

///
/// # Description
///
/// Response to a NEW message request.
///
/// This response contains the information needed for clients to interact with the newly
/// created User VM instance.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct NewResponse {
    /// Unique identifier assigned to the User VM instance.
    pub user_vm_id: UserVmIdentifier,
    /// Socket address where clients can interact with the VM's stdin/stdout through the gateway.
    pub gateway_sockaddr: String,
}

///
/// # Description
///
/// Message to terminate a running User VM instance.
///
/// This message is sent by external clients to request termination of an existing sandbox.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Kill {
    /// Unique identifier of the User VM instance to terminate.
    pub user_vm_id: UserVmIdentifier,
}

///
/// # Description
///
/// Response to a KILL message request.
///
/// This response indicates whether the termination was successful.
///
#[derive(Debug, Deserialize, Serialize)]
pub struct KillResponse {
    /// Exit code: 0 for success, non-zero for failure.
    pub exit_code: i32,
}

///
/// # Description
///
/// Unified response type for all message types.
///
/// This enum wraps the specific response types to allow returning different response
/// structures based on the request message type.
///
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageResponse {
    /// Response to a NEW message.
    New(NewResponse),
    /// Response to a KILL message.
    Kill(KillResponse),
}

///
/// # Description
///
/// Enumeration of supported message types.
///
/// This enum identifies the type of operation requested by the client and is transmitted
/// via the HTTP message type header.
///
#[derive(Debug)]
pub enum MessageType {
    /// NEW message type for creating a new sandbox.
    New,
    /// KILL message type for terminating an existing sandbox.
    Kill,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::New => write!(f, "NEW"),
            MessageType::Kill => write!(f, "KILL"),
        }
    }
}

impl std::str::FromStr for MessageType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "new" => Ok(Self::New),
            "kill" => Ok(Self::Kill),
            _ => Err(()),
        }
    }
}
