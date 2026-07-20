// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Per-descriptor flags.

//==================================================================================================
// Imports
//==================================================================================================

use ::fat32::Fat32Error;
use ::sysapi::{
    fcntl::file_descriptor_flags::{
        FD_CLOEXEC,
        FD_CLOFORK,
    },
    ffi::c_int,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Per-descriptor flags carried by a single file-descriptor slot.
///
/// POSIX requires these flags to be stored per descriptor rather than per open file description.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FdFlags(c_int);

//==================================================================================================
// Implementations
//==================================================================================================

impl FdFlags {
    /// Builds descriptor flags from raw bits.
    ///
    /// # Errors
    ///
    /// Returns [`Fat32Error::InvalidArgument`] if `raw` contains an unsupported bit.
    pub fn try_from_bits(raw: c_int) -> Result<Self, Fat32Error> {
        if raw & !(FD_CLOEXEC | FD_CLOFORK) != 0 {
            return Err(Fat32Error::InvalidArgument);
        }
        Ok(Self(raw))
    }

    /// Returns the raw flag bits.
    pub const fn bits(self) -> c_int {
        self.0
    }

    /// Returns whether close-on-exec is enabled.
    pub const fn close_on_exec(self) -> bool {
        self.0 & FD_CLOEXEC != 0
    }

    /// Returns whether close-on-fork is enabled.
    pub const fn close_on_fork(self) -> bool {
        self.0 & FD_CLOFORK != 0
    }

    /// Sets or clears close-on-exec.
    pub fn set_close_on_exec(&mut self, enable: bool) {
        self.set(FD_CLOEXEC, enable);
    }

    /// Sets or clears close-on-fork.
    pub fn set_close_on_fork(&mut self, enable: bool) {
        self.set(FD_CLOFORK, enable);
    }

    /// Sets or clears `flag` according to `enable`.
    fn set(&mut self, flag: c_int, enable: bool) {
        if enable {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}
