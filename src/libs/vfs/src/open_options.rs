// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Options for opening a file.

//==================================================================================================
// Imports
//==================================================================================================

use crate::file::{
    open_with_options,
    File,
};
use ::fat32::Fat32Error;

//==================================================================================================
// Structures
//==================================================================================================

/// Builder for opening files with specific access options.
///
/// Provides a readable, builder-pattern API for specifying file open modes.
///
/// # Default Behavior
///
/// If you call `open()` without setting any options, it defaults to read-only
/// mode (equivalent to `.read(true)`).
///
/// # Description
///
/// ```ignore
/// use vfs::{OpenOptions, File};
///
/// // Open for reading (implicit default)
/// let file = OpenOptions::new().open("/data/config.txt")?;
///
/// // Create a new file for writing
/// let file = OpenOptions::new()
///     .write(true)
///     .create(true)
///     .open("/data/output.txt")?;
///
/// // Create new file, fail if exists (O_CREAT | O_EXCL)
/// let file = OpenOptions::new()
///     .write(true)
///     .create_new(true)
///     .open("/data/unique.txt")?;
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl OpenOptions {
    /// Creates a new `OpenOptions` with all options set to false.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            read: false,
            write: false,
            create: false,
            create_new: false,
            truncate: false,
        }
    }

    /// Sets the option for read access.
    #[must_use]
    pub const fn read(mut self, read: bool) -> Self {
        self.read = read;
        self
    }

    /// Sets the option for write access.
    #[must_use]
    pub const fn write(mut self, write: bool) -> Self {
        self.write = write;
        self
    }

    /// Sets the option to create a new file if it doesn't exist.
    #[must_use]
    pub const fn create(mut self, create: bool) -> Self {
        self.create = create;
        self
    }

    /// Sets the option to truncate the file to zero length on open.
    ///
    /// Requires `write(true)`.
    #[must_use]
    pub const fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// Sets the option to create a new file, failing if it already exists.
    ///
    /// This is equivalent to `O_CREAT | O_EXCL` in POSIX terms.
    #[must_use]
    pub const fn create_new(mut self, create_new: bool) -> Self {
        self.create_new = create_new;
        self
    }

    /// Opens the file at the specified path with the configured options.
    ///
    /// If neither `read` nor `write` is set, defaults to `read(true)`.
    ///
    /// # Parameters
    ///
    /// - `path`: The path to the file to open.
    ///
    /// # Returns
    ///
    /// A new [`File`] handle, or an error.
    ///
    /// # Errors
    ///
    /// - [`Fat32Error::NotInitialized`] if the filesystem hasn't been
    ///   initialized.
    /// - [`Fat32Error::NotFound`] if the path doesn't exist and `create` is
    ///   false.
    /// - [`Fat32Error::ReadOnly`] if write/create/truncate on a read-only mount.
    /// - [`Fat32Error::InvalidArgument`] if `truncate` is set without `write`,
    ///   or if `create_new` is combined with `create` or `truncate`.
    /// - [`Fat32Error::AlreadyExists`] if `create_new` is set and file exists.
    pub fn open(self, path: &str) -> Result<File, Fat32Error> {
        // Validate: truncate requires write.
        if self.truncate && !self.write {
            return Err(Fat32Error::InvalidArgument);
        }

        // Validate: create_new is mutually exclusive with create and truncate.
        if self.create_new && (self.create || self.truncate) {
            return Err(Fat32Error::InvalidArgument);
        }

        // Default to read if neither read nor write specified.
        let read: bool = if !self.read && !self.write {
            true
        } else {
            self.read
        };

        open_with_options(path, read, self.write, self.create, self.create_new, self.truncate)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
