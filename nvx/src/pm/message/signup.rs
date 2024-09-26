// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ipc::{
        SystemMessage,
        SystemMessageHeader,
    },
    pm::message::{
        ProcessManagementMessage,
        ProcessManagementMessageHeader,
    },
};
use ::core::{
    ffi::CStr,
    mem,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::{
        Message,
        MessageType,
    },
    pm::ProcessIdentifier,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message that encodes a sing up operation
///
#[repr(C, packed)]
pub struct SignupMessage {
    /// Process identifier.
    pub pid: ProcessIdentifier,
    pub name: [u8; Self::NAME_SIZE],
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a signup message must match the size of a process management message payload.
::sys::static_assert_size!(SignupMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message that encodes the response of a signup operation.
///
#[repr(C, packed)]
pub struct SignupResponseMessage {
    /// Process identifier.
    pub pid: ProcessIdentifier,
    /// Status of the signup operation.
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a signup response message must match the size of a process management message payload.
::sys::static_assert_size!(SignupResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> SignupMessage {
    /// Size of name.
    pub const NAME_SIZE: usize = 16;
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - Self::NAME_SIZE;

    ///
    /// # Description
    ///
    /// Instantiates a new signup message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    ///
    pub fn new(pid: ProcessIdentifier, name: [u8; Self::NAME_SIZE]) -> Self {
        Self {
            pid,
            name,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a signup message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A signup message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a signup message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }

    ///
    /// # Description
    ///
    /// Returns the `pid` field encoded in the message.
    ///
    /// # Returns
    ///
    /// The `pid` field encoded in the message.
    ///
    pub fn pid(&self) -> ProcessIdentifier {
        self.pid
    }

    ///
    /// # Description
    ///
    /// Returns the `name` field encoded in the message.
    ///
    /// # Returns
    ///
    /// The `name` field encoded in the message.
    ///
    pub fn name(&'a self) -> Result<&'a CStr, Error> {
        match CStr::from_bytes_until_nul(&self.name) {
            Ok(name) => Ok(name),
            Err(_) => {
                Err(Error::new(ErrorCode::InvalidMessage, "invalid name field in signup message"))
            },
        }
    }
}

impl SignupResponseMessage {
    /// Size of padding.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Instantiates a new signup response message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    /// - `status`: Status of the signup operation.
    ///
    pub fn new(pid: ProcessIdentifier, status: i32) -> Self {
        Self {
            pid,
            status,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a signup response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A signup response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a signup response message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a signup message.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `name`: Process name.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Otherwise, an error is returned.
///
pub fn signup(pid: ProcessIdentifier, name: &str) -> Result<(), Error> {
    let name: [u8; SignupMessage::NAME_SIZE] = {
        let mut buffer: [u8; SignupMessage::NAME_SIZE] = [0; SignupMessage::NAME_SIZE];
        let name_bytes: &[u8] = name.as_bytes();
        let len: usize = core::cmp::min(name_bytes.len(), SignupMessage::NAME_SIZE);
        buffer[..len].copy_from_slice(&name_bytes[..len]);
        buffer
    };

    // Construct a signup message.
    let signup_message: SignupMessage = SignupMessage::new(pid, name);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Signup,
        signup_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC  message.
    let ipc_message: Message =
        Message::new(pid, ProcessIdentifier::PROCD, MessageType::Ipc, system_message.into_bytes());

    // Send IPC message.
    ::sys::kcall::ipc::send(&ipc_message)
}

///
/// # Description
///
/// Sends a signup response message.
///
/// # Parameters
///
/// - `destination`: Destination process.
/// - `pid`: Process identifier.
/// - `status`: Status of the signup operation.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Otherwise, an error is returned.
///
pub fn signup_response(
    destination: ProcessIdentifier,
    pid: ProcessIdentifier,
    status: i32,
) -> Result<(), Error> {
    // Construct a signup response message.
    let signup_response_message: SignupResponseMessage = SignupResponseMessage::new(pid, status);

    // Construct a process management message.
    let pm_message: ProcessManagementMessage = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::SignupResponse,
        signup_response_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC message.
    let ipc_message: Message = Message::new(
        ProcessIdentifier::PROCD,
        destination,
        MessageType::Ipc,
        system_message.into_bytes(),
    );

    // Send IPC message.
    ::sys::kcall::ipc::send(&ipc_message)
}
