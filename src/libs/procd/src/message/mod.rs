// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod lookup;
mod shutdown;
mod signup;

//==================================================================================================
// Exports
//==================================================================================================

pub use lookup::*;
pub use shutdown::*;
pub use signup::*;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::nvx::{
    ipc::SystemMessage,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Process Management Message Header
//==================================================================================================

///
/// # Description
///
/// A type that encodes the header of a process management message.
///
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ProcessManagementMessageHeader {
    /// Shutdown operation.
    Shutdown = 1,
    /// Signup operation.
    Signup = 2,
    /// Signup response.
    SignupResponse = 3,
    /// Lookup operation.
    Lookup = 4,
    /// Lookup response.
    LookupResponse = 5,
}

impl TryFrom<u8> for ProcessManagementMessageHeader {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ProcessManagementMessageHeader::Shutdown),
            2 => Ok(ProcessManagementMessageHeader::Signup),
            3 => Ok(ProcessManagementMessageHeader::SignupResponse),
            4 => Ok(ProcessManagementMessageHeader::Lookup),
            5 => Ok(ProcessManagementMessageHeader::LookupResponse),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid process management message")),
        }
    }
}

impl From<&ProcessManagementMessageHeader> for u8 {
    fn from(value: &ProcessManagementMessageHeader) -> Self {
        match value {
            ProcessManagementMessageHeader::Shutdown => 1,
            ProcessManagementMessageHeader::Signup => 2,
            ProcessManagementMessageHeader::SignupResponse => 3,
            ProcessManagementMessageHeader::Lookup => 4,
            ProcessManagementMessageHeader::LookupResponse => 5,
        }
    }
}

//==================================================================================================
// Process Management Message
//==================================================================================================

///
/// # Description
///
/// A type that encodes a process management message.
///
#[repr(C, packed)]
pub struct ProcessManagementMessage {
    /// Message header.
    pub header: ProcessManagementMessageHeader,
    /// Message payload.
    pub payload: [u8; Self::PAYLOAD_SIZE],
}

// NOTE: the size of a process management message must match the size of a system message payload.
::nvx::sys::static_assert_size!(ProcessManagementMessage, SystemMessage::PAYLOAD_SIZE);

impl ProcessManagementMessage {
    /// Size of payload.
    pub const PAYLOAD_SIZE: usize =
        SystemMessage::PAYLOAD_SIZE - mem::size_of::<ProcessManagementMessageHeader>();

    ///
    /// # Description
    ///
    /// Instantiates a new process management message.
    ///
    /// # Parameters
    ///
    /// - `header`: Message header.
    /// - `payload`: Message payload.
    ///
    /// # Returns
    ///
    /// A process management message.
    ///
    pub fn new(header: ProcessManagementMessageHeader, payload: [u8; Self::PAYLOAD_SIZE]) -> Self {
        Self { header, payload }
    }

    ///
    /// # Description
    ///
    /// Converts a byte array into a process management message.
    ///
    /// # Parameters
    ///
    /// - `bytes`: Byte array.
    ///
    /// # Returns
    ///
    /// A process management message.
    ///
    pub fn from_bytes(bytes: [u8; SystemMessage::PAYLOAD_SIZE]) -> Result<Self, Error> {
        // Check if message header is valid.
        let _header: ProcessManagementMessageHeader =
            ProcessManagementMessageHeader::try_from(bytes[0])?;

        let message: ProcessManagementMessage = unsafe { mem::transmute(bytes) };

        Ok(message)
    }

    ///
    /// # Description
    ///
    /// Converts a process management message into a byte array.
    ///
    /// # Returns
    ///
    /// The corresponding byte array.
    ///
    pub fn into_bytes(self) -> [u8; SystemMessage::PAYLOAD_SIZE] {
        unsafe { mem::transmute(self) }
    }
}
