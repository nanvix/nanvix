// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscalls;
        pub use self::syscalls::{
            mmap::mmap,
            mmap_reserve,
            mprotect::mprotect,
            munmap::munmap,
        };
        pub mod bindings;
    }
}

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    mm::{
        AccessPermission,
        ExecutePermission,
        ReadPermission,
        WritePermission,
    },
};
use ::sysapi::{
    ffi::c_int,
    sys_mman::{
        flags,
        prot_flags,
    },
};

//==================================================================================================

/// Protection options for memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct MemoryMapProtectionFlags {
    /// Bitwise OR of the protection flags.
    flags: c_int,
}
::static_assert::assert_eq_size!(MemoryMapProtectionFlags, core::mem::size_of::<c_int>());

impl MemoryMapProtectionFlags {
    /// Masks for memory protection flags.
    const MASK: c_int = prot_flags::PROT_NONE
        | prot_flags::PROT_READ
        | prot_flags::PROT_WRITE
        | prot_flags::PROT_EXEC;
}

impl TryFrom<c_int> for MemoryMapProtectionFlags {
    type Error = Error;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        if (value & !MemoryMapProtectionFlags::MASK) != 0 {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "invalid memory map protection flags",
            ));
        }
        Ok(MemoryMapProtectionFlags { flags: value })
    }
}

impl From<MemoryMapProtectionFlags> for AccessPermission {
    fn from(value: MemoryMapProtectionFlags) -> Self {
        let read: ReadPermission = if (value.flags & prot_flags::PROT_READ) != 0 {
            ReadPermission::Allow
        } else {
            ReadPermission::Deny
        };

        let write: WritePermission = if (value.flags & prot_flags::PROT_WRITE) != 0 {
            WritePermission::Allow
        } else {
            WritePermission::Deny
        };

        let execute: ExecutePermission = if (value.flags & prot_flags::PROT_EXEC) != 0 {
            ExecutePermission::Allow
        } else {
            ExecutePermission::Deny
        };

        AccessPermission::new(read, write, execute)
    }
}

impl From<MemoryMapProtectionFlags> for c_int {
    fn from(value: MemoryMapProtectionFlags) -> Self {
        value.flags
    }
}

/// Exclusive flags for memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExclusiveMemoryMapFlags {
    /// Map shared memory region.
    Shared = flags::MAP_SHARED,
    /// Map private memory region.
    Private = flags::MAP_PRIVATE,
}
::static_assert::assert_eq_size!(i32, core::mem::size_of::<c_int>());

impl ExclusiveMemoryMapFlags {
    /// Mask for exclusive memory map flags.
    const MASK: c_int = flags::MAP_SHARED | flags::MAP_PRIVATE;
}

impl TryFrom<c_int> for ExclusiveMemoryMapFlags {
    type Error = Error;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            flags::MAP_SHARED => Ok(ExclusiveMemoryMapFlags::Shared),
            flags::MAP_PRIVATE => Ok(ExclusiveMemoryMapFlags::Private),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid exclusive memory map flag")),
        }
    }
}

impl From<ExclusiveMemoryMapFlags> for c_int {
    fn from(value: ExclusiveMemoryMapFlags) -> Self {
        value as c_int
    }
}

/// Non exclusive flags for memory operations (can be OR'ed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum NonExclusiveMemoryMapFlags {
    /// Map memory region with fixed address.
    Fixed = flags::MAP_FIXED,
    /// Map memory region with anonymous allocation.
    Anonymous = flags::MAP_ANONYMOUS,
}
::static_assert::assert_eq_size!(i32, core::mem::size_of::<c_int>());

impl NonExclusiveMemoryMapFlags {
    /// Mask for non-exclusive memory map flags.
    pub const MASK: c_int = flags::MAP_FIXED | flags::MAP_ANONYMOUS;
}

impl TryFrom<c_int> for NonExclusiveMemoryMapFlags {
    type Error = Error;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        match value {
            flags::MAP_FIXED => Ok(NonExclusiveMemoryMapFlags::Fixed),
            flags::MAP_ANONYMOUS => Ok(NonExclusiveMemoryMapFlags::Anonymous),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid memory map flag")),
        }
    }
}

impl From<NonExclusiveMemoryMapFlags> for c_int {
    fn from(value: NonExclusiveMemoryMapFlags) -> Self {
        value as c_int
    }
}

/// Memory flags for memory operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryMapFlags {
    exclusive: ExclusiveMemoryMapFlags,
    non_exclusive: NonExclusiveMemoryMapFlags,
}
// Ensure that exclusive and non-exclusive flags do not overlap.
::static_assert::assert_eq!(
    (ExclusiveMemoryMapFlags::MASK & NonExclusiveMemoryMapFlags::MASK) == 0
);

impl MemoryMapFlags {
    /// Checks if the [`ExclusiveMemoryMapFlags::Shared`] flag is set.
    pub fn is_shared(&self) -> bool {
        self.exclusive == ExclusiveMemoryMapFlags::Shared
    }

    /// Checks if the [`ExclusiveMemoryMapFlags::Private`] flag is set.
    pub fn is_private(&self) -> bool {
        self.exclusive == ExclusiveMemoryMapFlags::Private
    }

    /// Checks if the [`NonExclusiveMemoryMapFlags::Fixed`] flag is set.
    pub fn is_fixed(&self) -> bool {
        (c_int::from(self.non_exclusive) & flags::MAP_FIXED) != 0
    }

    /// Checks if the [`NonExclusiveMemoryMapFlags::Anonymous`] flag is set.
    pub fn is_anonymous(&self) -> bool {
        (c_int::from(self.non_exclusive) & flags::MAP_ANONYMOUS) != 0
    }
}

impl TryFrom<c_int> for MemoryMapFlags {
    type Error = Error;

    fn try_from(value: c_int) -> Result<Self, Self::Error> {
        let exclusive: ExclusiveMemoryMapFlags =
            ExclusiveMemoryMapFlags::try_from(value & ExclusiveMemoryMapFlags::MASK)?;
        let non_exclusive: NonExclusiveMemoryMapFlags =
            NonExclusiveMemoryMapFlags::try_from(value & NonExclusiveMemoryMapFlags::MASK)?;
        Ok(MemoryMapFlags {
            exclusive,
            non_exclusive,
        })
    }
}
