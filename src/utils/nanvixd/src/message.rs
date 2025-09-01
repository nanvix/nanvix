// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! This file contains the messages used in the HTTP API between end-clients and nanvixd.

use ::serde::{
    Deserialize,
    Serialize,
};
use ::user_vm_api::RawUserVmIdentifier;

/// This message can be used to create a new User VM managed by this nanvixd
/// instance.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct New {
    pub tenant_id: String,
    pub app_name: String,
    pub program: String,
    pub program_args: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NewResponse {
    pub user_vm_id: RawUserVmIdentifier,
    /// UNIX socket where we can interact with the new VM's stdin/stdout.
    pub gateway_sockaddr: String,
}

/// This message can be used to kill a running VM.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Kill {
    pub user_vm_id: RawUserVmIdentifier,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KillResponse {
    pub exit_code: i32,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum MessageResponse {
    New(NewResponse),
    Kill(KillResponse),
}

#[derive(Debug)]
pub enum MessageType {
    New,
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
