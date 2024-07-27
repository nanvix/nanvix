// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents read access permission.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ReadPermission {
    /// Deny read access.
    Deny,
    /// Allow read access.
    Allow,
}

///
/// # Description
///
/// A type that represents write access permission.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WritePermission {
    /// Deny write access.
    Deny,
    /// Allow write access.
    Allow,
}

///
/// # Description
///
/// A type that represents execute access permission.
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ExecutePermission {
    /// Deny execute access.
    Deny,
    /// Allow execute access.
    Allow,
}

///
/// # Description
///
/// A type that represents access permissions.
///
#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub struct AccessPermission {
    /// Read permission.
    read: ReadPermission,
    /// Write permission.
    write: WritePermission,
    /// Execute permission.
    execute: ExecutePermission,
}

impl From<AccessPermission> for u8 {
    ///
    /// # Description
    ///
    /// Converts an [`AccessPermission`] to a [u8].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Returns the [u8] representation of the [`AccessPermission`].
    ///
    fn from(value: AccessPermission) -> Self {
        let mut result = 0;

        if value.read == ReadPermission::Allow {
            result |= 0b100;
        }

        if value.write == WritePermission::Allow {
            result |= 0b010;
        }

        if value.execute == ExecutePermission::Allow {
            result |= 0b001;
        }

        result
    }
}

impl TryFrom<u8> for AccessPermission {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to constructs an [`AccessPermission`] from a [u8].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Upon successful completion, an [`AccessPermission`] is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 0b111 {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "invalid value for access permission",
            ));
        }

        Ok(Self {
            read: if value & 0b100 != 0 {
                ReadPermission::Allow
            } else {
                ReadPermission::Deny
            },
            write: if value & 0b010 != 0 {
                WritePermission::Allow
            } else {
                WritePermission::Deny
            },
            execute: if value & 0b001 != 0 {
                ExecutePermission::Allow
            } else {
                ExecutePermission::Deny
            },
        })
    }
}

impl From<AccessPermission> for u16 {
    ///
    /// # Description
    ///
    /// Converts an [`AccessPermission`] to a [u16].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Returns the [u16] representation of the [`AccessPermission`].
    ///
    fn from(value: AccessPermission) -> Self {
        u16::from(u8::from(value))
    }
}

impl TryFrom<u16> for AccessPermission {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to constructs an [`AccessPermission`] from a [u16].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Upon successful completion, an [`AccessPermission`] is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::try_from(value as u8)
    }
}

impl From<AccessPermission> for u32 {
    ///
    /// # Description
    ///
    /// Converts an [`AccessPermission`] to a [u32].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Returns the [u32] representation of the [`AccessPermission`].
    ///
    fn from(value: AccessPermission) -> Self {
        u32::from(u8::from(value))
    }
}

impl TryFrom<u32> for AccessPermission {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to constructs an [`AccessPermission`] from a [u32].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Upon successful completion, an [`AccessPermission`] is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_from(value as u8)
    }
}

impl From<AccessPermission> for usize {
    ///
    /// # Description
    ///
    /// Converts an [`AccessPermission`] to a [usize].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Returns the [usize] representation of the [`AccessPermission`].
    ///
    fn from(value: AccessPermission) -> Self {
        usize::from(u8::from(value))
    }
}

impl TryFrom<usize> for AccessPermission {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to constructs an [`AccessPermission`] from a [usize].
    ///
    /// # Parameters
    ///
    /// * `value` - Value.
    ///
    /// # Returns
    ///
    /// Upon successful completion, an [`AccessPermission`] is returned. Upon failure, an error is
    /// returned instead.
    ///
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::try_from(value as u8)
    }
}

//==================================================================================================
// Implementations
//==================================================================================================

impl Default for ReadPermission {
    ///
    /// # Description
    ///
    /// Constructs a [`ReadPermission`] with default values.
    ///
    /// # Returns
    ///
    /// Returns a [`ReadPermission`] with [`ReadPermission::Deny`].
    ///
    fn default() -> Self {
        Self::Deny
    }
}

impl Default for WritePermission {
    ///
    /// # Description
    ///
    /// Constructs a [`WritePermission`] with default values.
    ///
    /// # Returns
    ///
    /// Returns a [`WritePermission`] with [`WritePermission::Deny`].
    ///
    fn default() -> Self {
        Self::Deny
    }
}

impl Default for ExecutePermission {
    ///
    /// # Description
    ///
    /// Constructs an [`ExecutePermission`] with default values.
    ///
    /// # Returns
    ///
    /// Returns an [`ExecutePermission`] with [`ExecutePermission::Deny`].
    ///
    fn default() -> Self {
        Self::Deny
    }
}

impl AccessPermission {
    ///
    /// # Description
    ///
    /// Constructs a [`AccessPermission`] with the given permissions.
    ///
    /// # Parameters
    ///
    /// * `read` - Read permission.
    /// * `write` - Write permission.
    /// * `execute` - Execute permission.
    ///
    /// # Returns
    ///
    /// Returns a [`AccessPermission`] with the given permissions.
    ///
    pub fn new(read: ReadPermission, write: WritePermission, execute: ExecutePermission) -> Self {
        Self {
            read,
            write,
            execute,
        }
    }

    ///
    /// # Description
    ///
    /// Asserts if read access is allowed.
    ///
    /// # Returns
    ///
    /// Returns `true` if read access is allowed, `false` otherwise.
    ///
    pub fn is_readable(&self) -> bool {
        match self.read {
            ReadPermission::Allow => true,
            ReadPermission::Deny => false,
        }
    }

    ///
    /// # Description
    ///
    /// Asserts if write access is allowed.
    ///
    /// # Returns
    ///
    /// Returns `true` if write access is allowed, `false` otherwise.
    ///
    pub fn is_writable(&self) -> bool {
        match self.write {
            WritePermission::Allow => true,
            WritePermission::Deny => false,
        }
    }

    ///
    /// # Description
    ///
    /// Asserts if execute access is allowed.
    ///
    /// # Returns
    ///
    /// Returns `true` if execute access is allowed, `false` otherwise.
    ///
    pub fn is_executable(&self) -> bool {
        match self.execute {
            ExecutePermission::Allow => true,
            ExecutePermission::Deny => false,
        }
    }

    ///
    /// # Description
    ///
    /// Constructs a [`AccessPermission`] with read-only access permission.
    ///
    /// # Returns
    ///
    /// Returns a [`AccessPermission`] with read-only access permission.
    ///
    pub const RDONLY: Self = Self {
        read: ReadPermission::Allow,
        write: WritePermission::Deny,
        execute: ExecutePermission::Deny,
    };

    ///
    /// # Description
    ///
    /// Constructs a [`AccessPermission`] with write-only access permission.
    ///
    /// # Returns
    ///
    /// Returns a [`AccessPermission`] with write-only access permission.
    ///
    pub const WRONLY: Self = Self {
        read: ReadPermission::Deny,
        write: WritePermission::Allow,
        execute: ExecutePermission::Deny,
    };

    ///
    /// # Description
    ///
    /// Constructs a [`AccessPermission`] with read and write access permissions.
    ///
    /// # Returns
    ///
    /// Returns a [`AccessPermission`] with read and write access permissions.
    ///
    pub const RDWR: Self = Self {
        read: ReadPermission::Allow,
        write: WritePermission::Allow,
        execute: ExecutePermission::Deny,
    };

    ///
    /// # Description
    ///
    /// Constructs a [`AccessPermission`] with read and execute access permissions.
    ///
    /// # Returns
    ///
    /// Returns a [`AccessPermission`] with read and execute access permissions.
    ///
    pub const EXEC: Self = Self {
        read: ReadPermission::Allow,
        write: WritePermission::Deny,
        execute: ExecutePermission::Allow,
    };
}

impl core::fmt::Debug for AccessPermission {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}{}{}",
            if self.read == ReadPermission::Allow {
                "r"
            } else {
                "-"
            },
            if self.write == WritePermission::Allow {
                "w"
            } else {
                "-"
            },
            if self.execute == ExecutePermission::Allow {
                "x"
            } else {
                "-"
            }
        )
    }
}
