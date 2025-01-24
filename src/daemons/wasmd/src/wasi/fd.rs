// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::wasi::{
    types::Errno,
    Fd,
    Prestat,
    Size,
    WasiCtxInner,
};
use ::alloc::string::String;

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    /// Closes a file descriptor.
    pub fn fd_close(&mut self, fd: Fd) -> Result<(), Errno> {
        // Find and remove file descriptor from the list of open files.
        let num_open_files: usize = self.files.len();
        self.files.retain(|file| file.fd() != fd);

        // Check if file descriptor was removed.
        if self.files.len() == num_open_files {
            return Err(Errno::Badf);
        }

        debug_assert!(self.files.len() == num_open_files - 1);

        Ok(())
    }

    /// Returns a description of the given pre-opened directory.
    pub fn fd_prestat_dir_get(&self, fd: Fd) -> Result<String, Errno> {
        // Search for file descriptor in the list of pre-open directories.
        for (dir, name) in &self.preopen_dirs {
            if dir.file().is_some() {
                match dir.fd() == fd {
                    true => return Ok(name.clone()),
                    false => (),
                }
            }
        }

        ::nvx::log!("fd_prestat_dir_get(): invalid file descriptor");
        Err(Errno::Badf)
    }

    /// Returns a description of the given pre-opened file descriptor.
    pub fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Errno> {
        // Search for file descriptor in the list of pre-open directories.
        for (dir, name) in &self.preopen_dirs {
            if dir.file().is_some() {
                match dir.fd() == fd {
                    true => return Ok(Prestat::new(Size::from(name.len()))),
                    false => (),
                }
            }
        }

        ::nvx::log!("fd_prestat_get(): invalid file descriptor");
        Err(Errno::Badf)
    }
}
