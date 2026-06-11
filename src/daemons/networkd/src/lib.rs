// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod dispatch;

//==================================================================================================
// Imports
//==================================================================================================

use ::net_backend::{
    error::NetError,
    HostFilter,
    NetBackend,
};
use ::sys::ipc::Message;
use ::syscall::{
    SystemCallMessage,
    SystemCallMessageHeader,
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
    /// the response message(s).
    ///
    /// # Parameters
    ///
    /// - `msg`: The incoming IKC message from the guest.
    ///
    /// # Returns
    ///
    /// On success, returns a vector of response messages to send back to the guest.
    /// On error (e.g., unrecognized header), returns `None`.
    ///
    pub fn handle_message(&self, msg: Message) -> Option<Vec<Message>> {
        let syscall_msg: SystemCallMessage = match SystemCallMessage::try_from_bytes(msg.payload) {
            Ok(m) => m,
            Err(_) => return None,
        };

        dispatch::dispatch_message(&self.backend, &self.filter, msg.source, syscall_msg)
    }

    ///
    /// # Description
    ///
    /// Returns `true` if the given message header corresponds to a networking system call that
    /// this daemon handles.
    ///
    pub fn is_networking_message(header: &SystemCallMessageHeader) -> bool {
        dispatch::is_networking_header(header)
    }
}

impl Default for NetworkDaemon {
    fn default() -> Self {
        Self::new(HostFilter::AllowAll).expect("platform networking initialization should succeed")
    }
}
