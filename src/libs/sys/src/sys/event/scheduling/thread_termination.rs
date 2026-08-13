// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ipc::Message,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
    ExitStatus,
};
use ::core::{
    fmt::Debug,
    mem,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure packs information about the termination of a thread.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ThreadTerminationInfo {
    /// Identifier of the process that owns the terminated thread.
    pub pid: ProcessIdentifier,
    /// Identifier of the thread that terminated.
    pub tid: ThreadIdentifier,
    /// Exit status of the thread that terminated.
    pub status: ExitStatus,
}
::static_assert::assert_eq_size!(ThreadTerminationInfo, 12);
::static_assert::assert_eq_align!(ThreadTerminationInfo, 4);
::static_assert::assert_eq!(mem::size_of::<ThreadTerminationInfo>() <= Message::PAYLOAD_SIZE);

//==================================================================================================
// Implementations
//==================================================================================================

impl ThreadTerminationInfo {
    ///
    /// # Description
    ///
    /// Creates a new [`ThreadTerminationInfo`] with the given information.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process that owns the terminated thread.
    /// - `tid`: Identifier of the thread that terminated.
    /// - `status`: Exit status of the thread that terminated.
    ///
    /// # Returns
    ///
    /// The new [`ThreadTerminationInfo`].
    ///
    pub fn new(pid: ProcessIdentifier, tid: ThreadIdentifier, status: ExitStatus) -> Self {
        Self { pid, tid, status }
    }

    ///
    /// # Description
    ///
    /// Returns the memory representation of the target [`ThreadTerminationInfo`] as a byte array
    /// in native byte order.
    ///
    /// # Returns
    ///
    /// The memory representation of the target [`ThreadTerminationInfo`] as a byte array in native
    /// byte order.
    ///
    pub fn to_ne_bytes(self) -> [u8; mem::size_of::<ThreadTerminationInfo>()] {
        let mut bytes: [u8; mem::size_of::<ThreadTerminationInfo>()] =
            [0; mem::size_of::<ThreadTerminationInfo>()];

        let mut offset: usize = 0;
        bytes[offset..offset + mem::size_of::<ProcessIdentifier>()]
            .copy_from_slice(&self.pid.to_ne_bytes());
        offset += mem::size_of::<ProcessIdentifier>();

        bytes[offset..offset + mem::size_of::<ThreadIdentifier>()]
            .copy_from_slice(&self.tid.to_ne_bytes());
        offset += mem::size_of::<ThreadIdentifier>();

        bytes[offset..offset + mem::size_of::<ExitStatus>()]
            .copy_from_slice(&self.status.to_ne_bytes());

        bytes
    }

    ///
    /// # Description
    ///
    /// Creates a new [`ThreadTerminationInfo`] from a byte array in native byte order.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array in native byte order.
    ///
    /// # Returns
    ///
    /// The new [`ThreadTerminationInfo`].
    ///
    pub fn from_ne_bytes(bytes: [u8; mem::size_of::<ThreadTerminationInfo>()]) -> Self {
        ::static_assert::assert_eq_size!(ProcessIdentifier, 4);
        ::static_assert::assert_eq_size!(ThreadIdentifier, 4);
        ::static_assert::assert_eq_size!(ExitStatus, 4);

        let mut offset: usize = 0;
        let pid: ProcessIdentifier = ProcessIdentifier::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += mem::size_of::<ProcessIdentifier>();

        let tid: ThreadIdentifier = ThreadIdentifier::from_ne_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += mem::size_of::<ThreadIdentifier>();

        let status: ExitStatus = ExitStatus::from_ne_bytes(&[
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);

        Self { pid, tid, status }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_termination_info_round_trip() {
        // Each field carries a distinct value, so a field that the native-endian layout drops or
        // overlaps is caught rather than masked by a zero byte pattern.
        for status in [ExitStatus::ok(), ExitStatus::from(0x0102_0304_u32)] {
            let info: ThreadTerminationInfo = ThreadTerminationInfo::new(
                ProcessIdentifier::from(5),
                ThreadIdentifier::from(7),
                status,
            );

            assert_eq!(ThreadTerminationInfo::from_ne_bytes(info.to_ne_bytes()), info);
        }
    }
}
