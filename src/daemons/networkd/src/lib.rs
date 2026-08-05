// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatch;
#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub(crate) mod epoll;
#[cfg(target_os = "linux")]
pub mod framing;
pub mod wire;

//==================================================================================================
// Imports
//==================================================================================================

use ::net_backend::{
    error::NetError,
    HostFilter,
    NetBackend,
};
use ::sys::ipc::{
    Message,
    MessageReceiver,
    MessageSender,
    RequestIdentifier,
};
use ::syscall::{
    SystemCallMessage,
    SystemCallMessageKind,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Network daemon for handling networking system calls.
///
/// In embedded mode (Phase 4), this struct is instantiated directly by the host-side runtime
/// and processes networking IKC messages via [`NetworkDaemon::handle_message`].
///
pub struct NetworkDaemon {
    /// Platform-agnostic networking backend.
    backend: NetBackend,
    /// Host egress filter applied to guest `connect()` destinations.
    filter: HostFilter,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NetworkDaemon {
    fn prepare_response(
        source: MessageSender,
        request_id: RequestIdentifier,
        response: &mut Message,
    ) {
        response.destination = MessageReceiver::new(source.pid, source.tid);
        request_id.write_to(response);
    }

    ///
    /// # Description
    ///
    /// Creates a new `NetworkDaemon` instance.
    ///
    /// `filter` is the host egress policy enforced on guest `connect()`
    /// destinations. Pass [`HostFilter::AllowAll`] for unrestricted networking.
    ///
    /// Returns an error if platform networking initialization fails.
    ///
    pub fn new(filter: HostFilter) -> Result<Self, NetError> {
        Ok(Self {
            backend: NetBackend::new()?,
            filter,
        })
    }

    ///
    /// # Description
    ///
    /// Processes a single IKC message containing a networking system call request and returns
    /// the response message.
    ///
    /// # Parameters
    ///
    /// - `msg`: The incoming IKC message from the guest.
    ///
    /// # Returns
    ///
    /// On success, returns the response message to send back to the guest.
    /// Returns `None` if the payload cannot be parsed into a system call message, or if
    /// [`dispatch::dispatch_message`] rejects the request (e.g., unrecognized header or
    /// missing thread identifier).
    ///
    pub fn handle_message(&self, msg: Message) -> Option<Message> {
        let source: MessageSender = msg.source;
        let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(msg.payload) {
            Ok(m) => m,
            Err(_) => return None,
        };
        let request_id: RequestIdentifier = syscall_msg.request_id();

        dispatch::dispatch_message(&self.backend, &self.filter, source, syscall_msg).map(
            |mut response| {
                Self::prepare_response(source, request_id, &mut response);
                response
            },
        )
    }

    ///
    /// # Description
    ///
    /// Processes a `send()` request whose payload was delivered out-of-band via a scatter/gather
    /// push.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `SendSocketRequest` system call message.
    /// - `data`: The payload pulled from the caller.
    ///
    /// # Returns
    ///
    /// The response message to send back to the guest.
    ///
    pub fn handle_send(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message {
        let request_id: RequestIdentifier = syscall_msg.request_id();
        let mut response: Message =
            dispatch::dispatch_send(&self.backend, source, syscall_msg, data);
        Self::prepare_response(source, request_id, &mut response);
        response
    }

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
    ///
    pub fn handle_sendto(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
        data: &[u8],
    ) -> Message {
        let request_id: RequestIdentifier = syscall_msg.request_id();
        let mut response: Message =
            dispatch::dispatch_sendto(&self.backend, &self.filter, source, syscall_msg, data);
        Self::prepare_response(source, request_id, &mut response);
        response
    }

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
    ///
    pub fn handle_recvfrom(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>) {
        let request_id: RequestIdentifier = syscall_msg.request_id();
        let (mut response, data): (Message, Vec<u8>) =
            dispatch::dispatch_recvfrom(&self.backend, source, syscall_msg);
        Self::prepare_response(source, request_id, &mut response);
        (response, data)
    }

    ///
    /// # Description
    ///
    /// Processes a `recv()` request whose payload is delivered out-of-band via a scatter/gather
    /// pull.
    ///
    /// # Parameters
    ///
    /// - `source`: Identifies the calling thread.
    /// - `syscall_msg`: The parsed `ReceiveSocketRequest` system call message.
    ///
    /// # Returns
    ///
    /// A tuple with the response message and the payload to push back to the guest.
    ///
    pub fn handle_recv(
        &self,
        source: MessageSender,
        syscall_msg: SystemCallMessage,
    ) -> (Message, Vec<u8>) {
        let request_id: RequestIdentifier = syscall_msg.request_id();
        let (mut response, data): (Message, Vec<u8>) =
            dispatch::dispatch_recv(&self.backend, source, syscall_msg);
        Self::prepare_response(source, request_id, &mut response);
        (response, data)
    }

    ///
    /// # Description
    ///
    /// Returns `true` if the given message kind corresponds to a networking system call that
    /// this daemon handles.
    ///
    pub fn is_networking_message(kind: &SystemCallMessageKind) -> bool {
        dispatch::is_networking_kind(kind)
    }
}

impl Default for NetworkDaemon {
    fn default() -> Self {
        Self::new(HostFilter::AllowAll).expect("platform networking initialization should succeed")
    }
}
