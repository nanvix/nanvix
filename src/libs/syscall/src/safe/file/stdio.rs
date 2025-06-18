// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    safe::RegularFile,
    unistd::syscall,
};
use ::core::mem::ManuallyDrop;
use ::sys::error::Error;
use ::sysapi::unistd::{
    STDERR_FILENO,
    STDIN_FILENO,
    STDOUT_FILENO,
};

//==================================================================================================
// Standard Input
//==================================================================================================

///
/// # Description
///
/// A structure that represents the standard input file.
///
pub struct StandardInput(ManuallyDrop<RegularFile>);

impl StandardInput {
    ///
    /// # Description
    ///
    /// Gets an instance of the standard input file.
    ///
    /// # Returns
    ///
    /// An instance of the standard input file.
    ///
    pub const fn get() -> Self {
        Self(ManuallyDrop::new(RegularFile::new(STDIN_FILENO)))
    }

    ///
    /// # Description
    ///
    /// Checks if the standard input refers to a terminal device.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a boolean indicating whether the standard input refers to a
    /// terminal device is returned. Otherwise, an error is returned instead.
    ///
    pub fn is_terminal(&self) -> Result<bool, Error> {
        syscall::isatty(self.0.as_raw_fd())
    }

    ///
    /// # Description
    ///
    /// Reads data from the standard input file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer to store the data.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes read is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.0.read(buf)
    }
}

//==================================================================================================
// Standard Output
//==================================================================================================

///
/// # Description
///
/// A structure that represents the standard output file.
///
pub struct StandardOutput(ManuallyDrop<RegularFile>);

impl StandardOutput {
    ///
    /// # Description
    ///
    /// Gets an instance of the standard output file.
    ///
    /// # Returns
    ///
    /// An instance of the standard output file.
    ///
    pub const fn get() -> Self {
        Self(ManuallyDrop::new(RegularFile::new(STDOUT_FILENO)))
    }

    ///
    /// # Description
    ///
    /// Checks if the standard output refers to a terminal device.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a boolean indicating whether the standard output refers to a
    /// terminal device is returned. Otherwise, an error is returned instead.
    ///
    pub fn is_terminal(&self) -> Result<bool, Error> {
        syscall::isatty(self.0.as_raw_fd())
    }

    ///
    /// # Description
    ///
    /// Synchronizes the standard output file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn synchronize(&mut self) -> Result<(), Error> {
        self.0.synchronize()
    }

    ///
    /// # Description
    ///
    /// Writes data to the standard output file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer containing the data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes written is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.0.write(buf)
    }
}

//==================================================================================================
// Standard Error
//==================================================================================================

///
/// # Description
///
/// A structure that represents the standard error file.
///
pub struct StandardError(ManuallyDrop<RegularFile>);

impl StandardError {
    ///
    /// # Description
    ///
    /// Gets an instance of the standard error file.
    ///
    /// # Returns
    ///
    /// An instance of the standard error file.
    ///
    pub const fn get() -> Self {
        Self(ManuallyDrop::new(RegularFile::new(STDERR_FILENO)))
    }

    ///
    /// # Description
    ///
    /// Checks if the standard error refers to a terminal device.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a boolean indicating whether the standard error refers to a
    /// terminal device is returned. Otherwise, an error is returned instead.
    ///
    pub fn is_terminal(&self) -> Result<bool, Error> {
        syscall::isatty(self.0.as_raw_fd())
    }

    ///
    /// # Description
    ///
    /// Synchronizes the standard error file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn synchronize(&mut self) -> Result<(), Error> {
        self.0.synchronize()
    }
    ///
    /// # Description
    ///
    /// Writes data to a the standard error file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer containing the data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes written is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        self.0.write(buf)
    }
}
