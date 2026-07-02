// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

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

///
/// # Description
///
/// Transport-agnostic seam over the networking daemon.
///
/// The standalone I/O handler talks to the network daemon exclusively through owned buffers
/// (never guest-local memory addresses). This trait captures exactly that owned-buffer contract so
/// the same demux logic can be served either by an in-process [`NetworkDaemon`] ([`LocalNetwork`])
/// or, in the future, by a decoupled `networkd` process reached over a socket.
///
/// The three methods mirror [`NetworkDaemon`] one-to-one and carry no guest addresses:
///
/// - [`NetworkTransport::handle_message`] handles inline socket operations (e.g. `send`).
/// - [`NetworkTransport::handle_sendto`] handles `sendto`, taking the already-drained payload.
/// - [`NetworkTransport::handle_recvfrom`] handles `recvfrom`, returning the payload to push back.
///
/// Implementations are called from blocking worker threads (`spawn_blocking`), so they must be
/// `Send + Sync`.
///
pub trait NetworkTransport: Send + Sync {
    ///
    /// # Description
    ///
    /// Handles an inline networking [`Message`] and returns any response messages.
    ///
    /// # Parameters
    ///
    /// - `msg`: The networking request message.
    ///
    /// # Returns
    ///
    /// `Some` with the response messages to forward to the guest, or `None` if the request produced
    /// no response.
    ///
    fn handle_message(&self, msg: Message) -> Option<Vec<Message>>;

    ///
    /// # Description
    ///
    /// Handles a `sendto` request whose bulk payload has already been drained from the transport.
    ///
    /// # Parameters
    ///
    /// - `source`: The originating guest endpoint.
    /// - `syscall_msg`: The decoded `sendto` system-call message.
    /// - `data`: The payload bytes to send.
    ///
    /// # Returns
    ///
    /// The response [`Message`] to forward to the guest.
    ///
    fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message;

    ///
    /// # Description
    ///
    /// Handles a `recvfrom` request, returning both the response message and the received payload.
    ///
    /// # Parameters
    ///
    /// - `source`: The originating guest endpoint.
    /// - `syscall_msg`: The decoded `recvfrom` system-call message.
    ///
    /// # Returns
    ///
    /// A tuple of the response [`Message`] and the received payload bytes to push back to the guest.
    ///
    fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>);
}

//==================================================================================================
// LocalNetwork
//==================================================================================================

///
/// # Description
///
/// In-process [`NetworkTransport`] implementation that owns a [`NetworkDaemon`] directly.
///
/// This preserves the standalone behavior byte-for-byte: every call is forwarded straight to the
/// wrapped daemon with no serialization or copying beyond what the daemon already performs.
///
pub struct LocalNetwork {
    /// The wrapped, in-process network daemon.
    daemon: NetworkDaemon,
}

impl LocalNetwork {
    ///
    /// # Description
    ///
    /// Creates a new in-process network transport.
    ///
    /// # Parameters
    ///
    /// - `filter`: The host egress filter applied to guest `connect()` destinations.
    ///
    /// # Returns
    ///
    /// On success, the new [`LocalNetwork`]. On failure, a human-readable error string describing
    /// why the underlying [`NetworkDaemon`] could not be initialized.
    ///
    pub fn new(filter: HostFilter) -> Result<Self, String> {
        NetworkDaemon::new(filter)
            .map(|daemon| Self { daemon })
            .map_err(|e| e.to_string())
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
