// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pm::ProcessIdentifier,
    ExitStatus,
};
use ::core::fmt::Debug;

//==================================================================================================
// Enumerations
//==================================================================================================

///
/// # Description
///
/// Role of a process in the system, as determined authoritatively by the kernel. The role is
/// carried in the process-termination event so that subscribers (e.g. the process manager daemon)
/// can route a termination without re-inferring what the process was from prior, race-prone state.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum ProcessRole {
    /// The init process: the non-daemon process spawned directly by the kernel. Its termination
    /// triggers system shutdown.
    Init = 0,
    /// A system daemon spawned directly by the kernel and identified by a well-known process
    /// identifier. Its termination deregisters it (and triggers a crash shutdown on failure).
    Daemon = 1,
    /// A user process forked from another process. Its termination is reaped.
    User = 2,
}

impl ProcessRole {
    ///
    /// # Description
    ///
    /// Classifies a process from its identifier and the identifier of its parent. This is the
    /// authoritative classification owned by the kernel, which spawns the init process and the
    /// daemons directly and owns the well-known daemon process identifiers.
    ///
    /// # Parameters
    ///
    /// - `pid`: Identifier of the process to classify.
    /// - `parent`: Identifier of the parent of the process to classify.
    ///
    /// # Returns
    ///
    /// The role of the process.
    ///
    pub fn classify(pid: ProcessIdentifier, parent: ProcessIdentifier) -> Self {
        if pid == ProcessIdentifier::PROCD
            || pid == ProcessIdentifier::MEMD
            || pid == ProcessIdentifier::VFSD
        {
            // A well-known daemon process identifier. The daemon check takes precedence over the
            // lineage check below, because daemons are also spawned directly by the kernel.
            Self::Daemon
        } else if parent == ProcessIdentifier::KERNEL {
            // A non-daemon process spawned directly by the kernel is the init process.
            Self::Init
        } else {
            // Any other process was forked from a user process.
            Self::User
        }
    }

    /// Returns the raw `u32` representation of the target [`ProcessRole`].
    pub(crate) fn to_u32(self) -> u32 {
        self as u32
    }

    /// Creates a [`ProcessRole`] from its raw `u32` representation. An unrecognized value is mapped
    /// to [`ProcessRole::User`], the role that triggers no system-wide action, so that a malformed
    /// event cannot spuriously shut the system down.
    pub(crate) fn from_u32(raw: u32) -> Self {
        match raw {
            0 => Self::Init,
            1 => Self::Daemon,
            _ => Self::User,
        }
    }
}

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
    pub status: ExitStatus,
    /// Identifier of the parent of the process that terminated.
    pub parent: ProcessIdentifier,
    /// Role of the process that terminated.
    pub role: ProcessRole,
}
::static_assert::assert_eq_size!(ProcessTerminationInfo, 16);
::static_assert::assert_eq_align!(ProcessTerminationInfo, 4);

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
    /// - `parent`: Identifier of the parent of the process that terminated.
    /// - `role`: Role of the process that terminated.
    ///
    /// # Returns
    ///
    /// The new [`ProcessTerminationInfo`].
    ///
    pub fn new(
        pid: ProcessIdentifier,
        status: ExitStatus,
        parent: ProcessIdentifier,
        role: ProcessRole,
    ) -> Self {
        Self {
            pid,
            status,
            parent,
            role,
        }
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

        bytes[offset..offset + core::mem::size_of::<ExitStatus>()]
            .copy_from_slice(&self.status.to_ne_bytes());
        offset += core::mem::size_of::<ExitStatus>();

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

        let status: ExitStatus = ExitStatus::from_ne_bytes(&[
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]);
        offset += core::mem::size_of::<ExitStatus>();

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

        Self {
            pid,
            status,
            parent,
            role,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_daemon_by_well_known_pid() {
        assert_eq!(
            ProcessRole::classify(ProcessIdentifier::MEMD, ProcessIdentifier::KERNEL),
            ProcessRole::Daemon
        );
        assert_eq!(
            ProcessRole::classify(ProcessIdentifier::VFSD, ProcessIdentifier::KERNEL),
            ProcessRole::Daemon
        );
        assert_eq!(
            ProcessRole::classify(ProcessIdentifier::PROCD, ProcessIdentifier::KERNEL),
            ProcessRole::Daemon
        );
    }

    #[test]
    fn classify_init_by_kernel_lineage() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(4);
        assert_eq!(ProcessRole::classify(pid, ProcessIdentifier::KERNEL), ProcessRole::Init);
    }

    #[test]
    fn classify_user_by_non_kernel_parent() {
        let pid: ProcessIdentifier = ProcessIdentifier::from(5);
        let parent: ProcessIdentifier = ProcessIdentifier::from(4);
        assert_eq!(ProcessRole::classify(pid, parent), ProcessRole::User);
    }

    #[test]
    fn termination_info_round_trip() {
        let info: ProcessTerminationInfo = ProcessTerminationInfo::new(
            ProcessIdentifier::from(5),
            ExitStatus::ok(),
            ProcessIdentifier::from(4),
            ProcessRole::User,
        );
        assert_eq!(ProcessTerminationInfo::from_ne_bytes(info.to_ne_bytes()), info);
    }
}
