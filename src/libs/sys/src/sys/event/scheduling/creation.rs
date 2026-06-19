// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::ProcessRole;
use crate::pm::ProcessIdentifier;
use ::core::fmt::Debug;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs information about the creation of a process.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ProcessCreationInfo {
    /// Identifier of the newly created process.
    pub pid: ProcessIdentifier,
    /// Identifier of the process that created it.
    pub parent: ProcessIdentifier,
    /// Role of the newly created process.
    pub role: ProcessRole,
}
::static_assert::assert_eq_size!(ProcessCreationInfo, 12);
::static_assert::assert_eq_align!(ProcessCreationInfo, 4);

//==================================================================================================
// Implementations
//==================================================================================================

impl ProcessCreationInfo {
    ///
    /// # Description
    ///
    /// Creates a new [`ProcessCreationInfo`] with the given information.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the newly created process.
    /// - `parent`: Identifier of the process that created it.
    /// - `role`: Role of the newly created process.
    ///
    /// # Returns
    ///
    /// The new [`ProcessCreationInfo`].
    ///
    pub fn new(pid: ProcessIdentifier, parent: ProcessIdentifier, role: ProcessRole) -> Self {
        Self { pid, parent, role }
    }

    ///
    /// # Description
    ///
    /// Returns the memory representation of the target [`ProcessCreationInfo`] as a byte array in
    /// native byte order.
    ///
    /// # Returns
    ///
    /// The memory representation of the target [`ProcessCreationInfo`] as a byte array in native
    /// byte order.
    ///
    pub fn to_ne_bytes(self) -> [u8; core::mem::size_of::<ProcessCreationInfo>()] {
        let mut bytes: [u8; core::mem::size_of::<ProcessCreationInfo>()] =
            [0; core::mem::size_of::<ProcessCreationInfo>()];

        let mut offset: usize = 0;
        bytes[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&self.pid.to_ne_bytes());
        offset += core::mem::size_of::<ProcessIdentifier>();

        bytes[offset..offset + core::mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&self.parent.to_ne_bytes());
        offset += core::mem::size_of::<ProcessIdentifier>();

        bytes[offset..offset + core::mem::size_of::<u32>()]
            .copy_from_slice(&self.role.to_u32().to_ne_bytes());

        bytes
    }

    ///
    /// # Description
    ///
    /// Creates a new [`ProcessCreationInfo`] from a byte array in native byte order.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array in native byte order.
    ///
    /// # Returns
    ///
    /// The new [`ProcessCreationInfo`].
    ///
    pub fn from_ne_bytes(bytes: [u8; core::mem::size_of::<ProcessCreationInfo>()]) -> Self {
        ::static_assert::assert_eq_size!(ProcessIdentifier, 4);

        let mut offset: usize = 0;
        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += core::mem::size_of::<ProcessIdentifier>();

        let parent: ProcessIdentifier = ProcessIdentifier::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += core::mem::size_of::<ProcessIdentifier>();

        let role: ProcessRole = ProcessRole::from_u32(u32::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]));

        Self { pid, parent, role }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_info_round_trip() {
        let info: ProcessCreationInfo = ProcessCreationInfo::new(
            ProcessIdentifier::from(5),
            ProcessIdentifier::from(4),
            ProcessRole::Init,
        );
        assert_eq!(ProcessCreationInfo::from_ne_bytes(info.to_ne_bytes()), info);
    }
}
