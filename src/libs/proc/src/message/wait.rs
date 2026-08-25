// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::core::mem;
use ::sys::{
    error::Error,
    ipc::{
        Message,
        MessageReceiver,
        MessageSender,
        MessageType,
        RequestIdentifier,
        SystemMessage,
        SystemMessageHeader,
    },
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Selects which child a wait operation targets, following POSIX `waitpid()` semantics.
///
pub enum WaitTarget {
    /// Wait for any child of the caller.
    Any,
    /// Wait for the specific child with the given process identifier.
    Pid(ProcessIdentifier),
}

///
/// # Description
///
/// A message that encodes a wait operation. It is sent by a process to the process manager daemon
/// to wait for the termination of a child process (`waitpid()`).
///
#[repr(C, packed)]
pub struct WaitMessage {
    /// Wire encoding of the child selector. This is a raw `i32` rather than a [`ProcessIdentifier`]
    /// because the non-positive selector values that POSIX `waitpid()` allows (`-1`, `0`, `< -1`)
    /// are not valid process identifiers. The field is private so callers cannot build an invalid
    /// selector directly: use [`WaitMessage::new`] to construct one from a [`WaitTarget`] and
    /// [`WaitMessage::target`] to decode it back.
    selector: i32,
    /// Wait options.
    pub options: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a wait message must match the size of a process management message payload.
::static_assert::assert_eq_size!(WaitMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a wait operation.
///
#[repr(C, packed)]
pub struct WaitResponseMessage {
    /// Process identifier of the reaped child (`0` when no child was ready and `WNOHANG` was set).
    pub child: ProcessIdentifier,
    /// Termination status of the reaped child.
    pub status: i32,
    /// Error code of the wait operation (`0` on success).
    pub error: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a wait response message must match the size of a process management message payload.
::static_assert::assert_eq_size!(WaitResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

/// Identifies a blocked wait request to cancel.
#[repr(C, packed)]
pub struct WaitCancelMessage {
    request_id: u32,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(WaitCancelMessage, ProcessManagementMessage::PAYLOAD_SIZE);

/// Reports whether a wait cancellation won the race with completion.
#[repr(C, packed)]
pub struct WaitCancelResponseMessage {
    cancelled: u8,
    _padding: [u8; Self::PADDING_SIZE],
}
::static_assert::assert_eq_size!(WaitCancelResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl WaitTarget {
    /// Encodes this selector into its wire representation. [`WaitTarget::Any`] is encoded as `-1`,
    /// because process groups are not supported yet.
    pub fn into_raw(self) -> i32 {
        match self {
            WaitTarget::Any => -1,
            WaitTarget::Pid(pid) => i32::from(pid),
        }
    }

    /// Decodes a wire selector. A positive value selects that specific child; any non-positive
    /// value (`-1`, `0`, or `< -1`) selects any child of the caller. The POSIX process-group
    /// selectors (`0` and `< -1`) are folded into [`WaitTarget::Any`] because Nanvix has no process
    /// groups yet.
    pub fn from_raw(raw: i32) -> Self {
        if raw > 0 {
            WaitTarget::Pid(ProcessIdentifier::from(raw))
        } else {
            WaitTarget::Any
        }
    }
}

impl WaitMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize =
        ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<i32>() - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new wait message.
    ///
    /// # Parameters
    ///
    /// - `target`: Selects which child to wait for.
    /// - `options`: Wait options.
    ///
    pub fn new(target: WaitTarget, options: i32) -> Self {
        Self {
            selector: target.into_raw(),
            options,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Decodes the strongly-typed child selector carried by this message.
    ///
    /// # Returns
    ///
    /// The wait target that this message selects.
    ///
    pub fn target(&self) -> WaitTarget {
        // Copy the field out of the packed struct before decoding to avoid an unaligned reference.
        let selector: i32 = self.selector;
        WaitTarget::from_raw(selector)
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a wait message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A wait message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a wait message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl WaitResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new wait response message.
    ///
    /// # Parameters
    ///
    /// - `child`: Process identifier of the reaped child.
    /// - `status`: Termination status of the reaped child.
    /// - `error`: Error code of the wait operation.
    ///
    pub fn new(child: ProcessIdentifier, status: i32, error: i32) -> Self {
        Self {
            child,
            status,
            error,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a wait response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A wait response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a wait response message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

impl WaitCancelMessage {
    const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<u32>();

    /// Creates a cancellation request for `request_id`.
    pub fn new(request_id: RequestIdentifier) -> Self {
        Self {
            request_id: request_id.raw(),
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a wait cancellation request.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes a wait cancellation request.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Returns the request identifier to cancel.
    pub fn request_id(&self) -> RequestIdentifier {
        RequestIdentifier::from_raw(self.request_id)
    }
}

impl WaitCancelResponseMessage {
    const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - mem::size_of::<u8>();

    /// Creates a wait cancellation response.
    pub fn new(cancelled: bool) -> Self {
        Self {
            cancelled: u8::from(cancelled),
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    /// Deserializes a wait cancellation response.
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    /// Serializes a wait cancellation response.
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    /// Returns whether the wait request was cancelled before completion.
    pub fn cancelled(&self) -> bool {
        self.cancelled != 0
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Builds a wait request message.
///
/// # Parameters
///
/// - `caller`: Process identifier of the calling process (and message sender).
/// - `target`: Selects which child to wait for.
/// - `options`: Wait options.
///
/// # Returns
///
/// Upon successful completion, a wait request message is returned. Otherwise, an error is returned
/// instead.
///
pub fn wait_request(
    caller: ProcessIdentifier,
    target: WaitTarget,
    options: i32,
) -> Result<Message, Error> {
    // Reject invalid `Pid` selectors: on the wire, any non-positive value is treated as "any child".
    if let WaitTarget::Pid(pid) = &target {
        if i32::from(*pid) <= 0 {
            return Err(Error::new(
                ::sys::error::ErrorCode::InvalidArgument,
                "invalid wait target pid",
            ));
        }
    }

    // Construct a wait message.
    let wait_message: WaitMessage = WaitMessage::new(target, options);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Wait,
        wait_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::new(caller, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

///
/// # Description
///
/// Builds a wait response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `child`: Process identifier of the reaped child.
/// - `status`: Termination status of the reaped child.
/// - `error`: Error code of the wait operation.
///
/// # Returns
///
/// Upon successful completion, a wait response message is returned. Otherwise, an error is returned
/// instead.
///
pub fn wait_response(
    destination: ProcessIdentifier,
    child: ProcessIdentifier,
    status: i32,
    error: i32,
) -> Result<Message, Error> {
    // Construct a wait response message.
    let wait_response_message: WaitResponseMessage = WaitResponseMessage::new(child, status, error);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::WaitResponse,
        wait_response_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        MessageSender::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageReceiver::new(destination, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    );

    Ok(ipc_message)
}

/// Builds a request to cancel one blocked wait operation.
pub fn wait_cancel_request(
    caller: ProcessIdentifier,
    request_id: RequestIdentifier,
) -> Result<Message, Error> {
    let cancel: WaitCancelMessage = WaitCancelMessage::new(request_id);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::WaitCancel,
        cancel.into_bytes(),
    );
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());
    Ok(Message::new(
        MessageSender::new(caller, ThreadIdentifier::NONE),
        MessageReceiver::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    ))
}

/// Builds the acknowledgement for a wait cancellation request.
pub fn wait_cancel_response(
    destination: ProcessIdentifier,
    cancelled: bool,
) -> Result<Message, Error> {
    let response: WaitCancelResponseMessage = WaitCancelResponseMessage::new(cancelled);
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::WaitCancelResponse,
        response.into_bytes(),
    );
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());
    Ok(Message::new(
        MessageSender::new(ProcessIdentifier::PROCD, ThreadIdentifier::NONE),
        MessageReceiver::new(destination, ThreadIdentifier::NONE),
        MessageType::Ipc,
        None,
        system_message.into_bytes(),
    ))
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wait_cancel_request_round_trip() {
        let request_id: RequestIdentifier = RequestIdentifier::from_raw(42);
        let request: WaitCancelMessage = WaitCancelMessage::new(request_id);
        let decoded: WaitCancelMessage = WaitCancelMessage::from_bytes(request.into_bytes());
        assert_eq!(decoded.request_id(), request_id);
    }

    #[test]
    fn wait_cancel_response_round_trip() {
        let response: WaitCancelResponseMessage = WaitCancelResponseMessage::new(true);
        let decoded: WaitCancelResponseMessage =
            WaitCancelResponseMessage::from_bytes(response.into_bytes());
        assert!(decoded.cancelled());
    }
}
