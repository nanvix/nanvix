// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    ProcessManagementMessage,
    ProcessManagementMessageHeader,
};
use ::core::{
    ffi::CStr,
    mem,
};
use ::nvx::{
    ipc::{
        SystemMessage,
        SystemMessageHeader,
    },
    sys::{
        error::{
            Error,
            ErrorCode,
        },
        ipc::{
            Message,
            MessageType,
        },
        pm::ProcessIdentifier,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A message tha encodes a lookup system call.
///
#[repr(C, packed)]
pub struct LookupMessage {
    /// Process name.
    pub name: [u8; Self::NAME_SIZE],
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a lookup message must match the size of a process management message payload.
::nvx::sys::static_assert_size!(LookupMessage, ProcessManagementMessage::PAYLOAD_SIZE);

///
/// # Description
///
/// A message the encodes the result of a lookup system call
///
#[repr(C, packed)]
pub struct LookupResponseMessage {
    /// Process identifier.
    pub pid: ProcessIdentifier,
    pub status: i32,
    _padding: [u8; Self::PADDING_SIZE],
}

// NOTE: The size of a lookup response message must match the size of a process management message payload.
::nvx::sys::static_assert_size!(LookupResponseMessage, ProcessManagementMessage::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> LookupMessage {
    /// Name size.
    pub const NAME_SIZE: usize = 16;
    /// Padding size.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE - Self::NAME_SIZE;

    ///
    /// # Description
    ///
    /// Creates a new lookup message.
    ///
    /// # Parameters
    ///
    /// - `name`: Name of the process.
    ///
    /// # Returns
    ///
    /// A new lookup message.
    ///
    pub fn new(name: [u8; Self::NAME_SIZE]) -> Self {
        Self {
            name,
            _padding: [0; Self::PADDING_SIZE],
        }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a lookup message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A lookup message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a lookup message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn as_bytes(self) -> [u8; ProcessManagementMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
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
                Err(Error::new(ErrorCode::InvalidMessage, "invalid name field in the message"))
            },
        }
    }
}

impl LookupResponseMessage {
    /// Padding size.
    pub const PADDING_SIZE: usize = ProcessManagementMessage::PAYLOAD_SIZE
        - mem::size_of::<ProcessIdentifier>()
        - mem::size_of::<i32>();

    ///
    /// # Description
    ///
    /// Creates a new lookup response message.
    ///
    /// # Parameters
    ///
    /// - `pid`: Process identifier.
    ///
    /// # Returns
    ///
    /// A new lookup response message.
    ///
    #[cfg(feature = "daemon")]
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
    /// Converts a byte array into a lookup response message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A lookup response message.
    ///
    pub fn from_bytes(bytes: [u8; ProcessManagementMessage::PAYLOAD_SIZE]) -> Self {
        unsafe { mem::transmute(bytes) }
    }

    ///
    /// # Description
    ///
    /// Converts a lookup response message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    #[cfg(feature = "daemon")]
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
/// Sends a lookup message.
///
/// # Parameters
///
/// - `name`: Name of the process.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned.
///
pub fn lookup(name: &str) -> Result<(), Error> {
    let name: [u8; LookupMessage::NAME_SIZE] = {
        let mut buffer = [0; LookupMessage::NAME_SIZE];
        let name_bytes = name.as_bytes();
        let name_len = core::cmp::min(name_bytes.len(), LookupMessage::NAME_SIZE);
        buffer[..name_len].copy_from_slice(&name_bytes[..name_len]);
        buffer
    };

    // Construct a lookup message.
    let lookup_message = LookupMessage::new(name);

    // Construct a process management message.
    let pm_message = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::Lookup,
        lookup_message.as_bytes(),
    );

    // FIXME: this should not be required.
    let mypid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC  message.
    let ipc_message: Message = Message::new(
        mypid,
        ProcessIdentifier::PROCD,
        MessageType::Ipc,
        system_message.into_bytes(),
    );

    // Send IPC message.
    ::nvx::ipc::send(&ipc_message)
}

///
/// # Description
///
/// Sends a lookup response message.
///
/// # Parameters
///
/// - `pid`: Process identifier.
/// - `status`: Status of the lookup operation.
///
/// # Returns
///
/// Upon successful completion, empty result is returned. Otherwise, an error is returned.
///
#[cfg(feature = "daemon")]
pub fn lookup_response(
    destination: ProcessIdentifier,
    pid: ProcessIdentifier,
    status: i32,
) -> Result<(), Error> {
    // Construct a lookup response message.
    let lookup_response_message = LookupResponseMessage::new(pid, status);

    // Construct a process management message.
    let pm_message = ProcessManagementMessage::new(
        ProcessManagementMessageHeader::LookupResponse,
        lookup_response_message.into_bytes(),
    );

    // Construct a system message.
    let system_message: SystemMessage =
        SystemMessage::new(SystemMessageHeader::ProcessManagement, pm_message.into_bytes());

    // Construct an IPC  message.
    let ipc_message: Message = Message::new(
        ProcessIdentifier::PROCD,
        destination,
        MessageType::Ipc,
        system_message.into_bytes(),
    );

    // Send IPC message.
    ::nvx::ipc::send(&ipc_message)
}
