// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Privilege level.
#[repr(u8)]
pub enum PrivilegeLevel {
    Ring0 = 0,
    Ring1 = 1,
    Ring2 = 2,
    Ring3 = 3,
}

impl core::fmt::Debug for PrivilegeLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            PrivilegeLevel::Ring0 => write!(f, "ring 0"),
            PrivilegeLevel::Ring1 => write!(f, "ring 1"),
            PrivilegeLevel::Ring2 => write!(f, "ring 2"),
            PrivilegeLevel::Ring3 => write!(f, "ring 3"),
        }
    }
}
