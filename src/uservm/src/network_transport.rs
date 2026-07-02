// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::nanvix_sandbox_config::HostFilter;
use ::networkd::NetworkDaemon;
use ::sys::ipc::{
    Message,
    MessageSender,
};
use ::syscall::SystemCallMessage;

//==================================================================================================
// NetworkTransport
//==================================================================================================

/// Abstracts where the network daemon runs.
///
/// Implementations handle standalone networking requests using host-owned buffers, whether the
/// daemon is in-process or reached through a local or remote process boundary.
pub trait NetworkTransport: Send + Sync {
    ///
    /// # Description
    ///
    /// Processes a single IKC message containing a networking system call request and returns
    /// the response message(s).
    ///
    /// # Parameters
    ///
    /// - `msg`: The incoming IKC message from the guest.
    ///
    /// # Returns
    ///
    /// On success, returns a vector of response messages to send back to the guest. On error (e.g.,
    /// unrecognized header), returns `None`.
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>>;

    ///
    /// # Description
    ///
    /// Processes a `sendto()` request whose datagram payload was delivered out-of-band via a
    /// scatter/gather push.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `SendToSocketRequest` system call message.
    /// - `data`: The datagram payload pulled from the caller.
    ///
    /// # Returns
    ///
    /// The response message to send back to the guest.
    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message;

    ///
    /// # Description
    ///
    /// Processes a `recvfrom()` request whose datagram payload is delivered out-of-band via a
    /// scatter/gather pull.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `ReceiveFromSocketRequest` system call message.
    ///
    /// # Returns
    ///
    /// A tuple with the response message and the datagram payload to push back to the guest.
    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>);
}

//==================================================================================================
// LocalNetwork
//==================================================================================================

/// In-process [`NetworkTransport`] backed by a [`NetworkDaemon`].
pub struct LocalNetwork {
    daemon: NetworkDaemon,
}

impl LocalNetwork {
    ///
    /// # Description
    ///
    /// Creates a network daemon-backed transport.
    ///
    /// # Parameters
    ///
    /// - `filter`: The host egress filter applied to guest `connect()` destinations.
    ///
    /// # Returns
    ///
    /// On success, the new [`LocalNetwork`]. On failure, returns the underlying initialization
    /// error from [`NetworkDaemon`].
    ///
    pub fn new(filter: HostFilter) -> Result<Self> {
        Ok(Self {
            daemon: NetworkDaemon::new(filter)?,
        })
    }
}

impl NetworkTransport for LocalNetwork {
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>> {
        self.daemon.handle_message(msg)
    }

    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message {
        self.daemon.handle_sendto(source, syscall_msg, data)
    }

    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>) {
        self.daemon.handle_recvfrom(source, syscall_msg)
    }
}
