// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::pm::ProcessIdentifier;
use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs information about the termination of a process.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ProcessTerminationInfo {
    /// Identifier of the process that terminated.
    pub pid: ProcessIdentifier,
    /// Exit status of the process that terminated.
    pub status: i32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessTerminationInfo {
    ///
    /// # Description
    ///
    /// Creates a new [`ProcessTerminationInfo`] with the given information.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that terminated.
    /// - `status`: Exit status of the process that terminated.
    ///
    /// # Returns
    ///
    /// The new [`ProcessTerminationInfo`].
    ///
    pub fn new(pid: ProcessIdentifier, status: i32) -> Self {
        Self { pid, status }
    }

    ///
    /// # Description
    ///
    /// Returns the memory memory representation of the target [`ProcessTerminationInfo`] as a byte
    /// array in native byte order.
    ///
    /// # Returns
    ///
    /// The memory representation of the target [`ProcessTerminationInfo`] as a byte array in native
    /// byte order.
    ///
    pub fn to_ne_bytes(self) -> [u8; core::mem::size_of::<ProcessTerminationInfo>()] {
        let mut bytes: [u8; core::mem::size_of::<ProcessTerminationInfo>()] =
            [0; core::mem::size_of::<ProcessTerminationInfo>()];

        let mut offset: usize = 0;
        bytes[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&self.pid.to_ne_bytes());
        offset += core::mem::size_of::<ProcessIdentifier>();

        bytes[offset..offset + core::mem::size_of::<i32>()]
            .copy_from_slice(&self.status.to_ne_bytes());

        bytes
    }

    ///
    /// # Description
    ///
    /// Creates a new [`ProcessTerminationInfo`] from a byte array in native byte order.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array in native byte order.
    ///
    /// # Returns
    ///
    /// The new [`ProcessTerminationInfo`].
    ///
    pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<ProcessTerminationInfo>()]) -> Self {
        let mut offset: usize = 0;
        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += core::mem::size_of::<ProcessIdentifier>();

        let status: i32 = i32::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);

        Self { pid, status }
    }
}
