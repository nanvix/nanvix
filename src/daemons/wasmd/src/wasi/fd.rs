// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    pal::fs::File,
    wasi::{
        types::{
            Errno,
            FileDelta,
            FileSize,
            IoVec,
            Pointer,
            Prestat,
            Size,
            Whence,
        },
        Fd,
        Slice,
        WasiCtxInner,
    },
};
use ::alloc::string::String;

//==================================================================================================
// Implementations
//==================================================================================================

impl WasiCtxInner {
    /// Closes a file descriptor.
    pub fn fd_close(&mut self, fd: Fd) -> Result<(), Errno> {
        // TODO: Check if trying to close stdin, stdout, or stderr.

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
            match dir.fd() == fd {
                true => return Ok(name.clone()),
                false => (),
            }
        }

        ::nvx::log!("fd_prestat_dir_get(): invalid file descriptor");
        Err(Errno::Badf)
    }

    /// Returns a description of the given pre-opened file descriptor.
    pub fn fd_prestat_get(&self, fd: Fd) -> Result<Prestat, Errno> {
        // Search for file descriptor in the list of pre-open directories.
        for (dir, name) in &self.preopen_dirs {
            match dir.fd() == fd {
                true => return Ok(Prestat::new(Size::from(name.len()))),
                false => (),
            }
        }

        ::nvx::log!("fd_prestat_get(): invalid file descriptor");
        Err(Errno::Badf)
    }

    /// Reads from a file descriptor.
    pub fn fd_read(&self, memory: &mut [u8], fd: Fd, iovecs: &[IoVec]) -> Result<Size, Errno> {
        let read_iovecs = |file: &File, memory: &mut [u8], dry_run: bool| -> Result<Size, Errno> {
            let mut total_read: Size = Size::new(0);
            for iovec in iovecs {
                let ptr: Pointer<u8> = iovec.buf();
                let len: Size = iovec.buf_len();

                let mut buf: Slice<'_, u8> = Slice::<u8>::for_raw_parts(memory, ptr, len);
                let buf: &mut [u8] = match buf.as_mut() {
                    Ok(slice) => slice,
                    Err(_) => {
                        ::nvx::log!("fd_read(): failed to get slice from memory");
                        return Err(Errno::Inval);
                    },
                };

                let read: Size = if !dry_run {
                    match file.read(buf) {
                        Ok(read) => read.into(),
                        Err(err) => {
                            ::nvx::log!("fd_read(): failed to read from file (errno={:?})", err);
                            return Err(Errno::from(err.value()));
                        },
                    }
                } else {
                    len
                };

                total_read += read;
            }
            Ok(total_read)
        };

        match self.get_file(fd) {
            Some(file) => {
                // Ensure that we have the right to invoke this operation.
                if !file.rights_base().fd_read {
                    ::nvx::log!("fd_read(): access denied");
                    return Err(Errno::Acces);
                }

                let file: &File = file.file();

                // Dry run to check for errors.
                read_iovecs(file, memory, true)?;
                // Normal run to read from the file.
                read_iovecs(file, memory, false)
            },
            None => {
                ::nvx::log!("fd_read(): invalid file descriptor");
                return Err(Errno::Badf);
            },
        }
    }

    /// Moves the offset of a file descriptor.
    pub fn fd_seek(
        &mut self,
        fd: Fd,
        offset: FileDelta,
        whence: Whence,
    ) -> Result<FileSize, Errno> {
        match self.get_file_mut(fd) {
            Some(file) => {
                // Ensure that we have the right to invoke this operation.
                if !file.rights_base().fd_seek {
                    ::nvx::log!("fd_seek(): access denied");
                    return Err(Errno::Acces);
                }

                match file
                    .file_mut()
                    .seek(offset.into(), whence.into())
                    .map_err(|err| Errno::from(err.value()))?
                    .try_into()
                {
                    Ok(offset) => Ok(offset),
                    Err(_) => {
                        ::nvx::log!("fd_seek(): failed to convert offset to FileSize");
                        return Err(Errno::TooBig);
                    },
                }
            },
            None => {
                ::nvx::log!("fd_seek(): invalid file descriptor");
                return Err(Errno::Badf);
            },
        }
    }

    /// Returns the current offset of a file descriptor.
    pub fn fd_tell(&self, fd: Fd) -> Result<FileSize, Errno> {
        match self.get_file(fd) {
            Some(file) => {
                // Ensure that we have the right to invoke this operation.
                if !file.rights_base().fd_seek {
                    ::nvx::log!("fd_tell(): access denied");
                    return Err(Errno::Acces);
                }

                match file
                    .file()
                    .tell()
                    .map_err(|err| Errno::from(err.value()))?
                    .try_into()
                {
                    Ok(offset) => Ok(offset),
                    Err(_) => {
                        ::nvx::log!("fd_tell(): failed to convert offset to FileSize");
                        return Err(Errno::TooBig);
                    },
                }
            },
            None => {
                ::nvx::log!("fd_tell(): invalid file descriptor");
                return Err(Errno::Badf);
            },
        }
    }

    /// Writes to a file descriptor.
    pub fn fd_write(&mut self, memory: &[u8], fd: Fd, iovecs: &[IoVec]) -> Result<Size, Errno> {
        let write_iovecs = |file: &mut File, dry_run: bool| -> Result<Size, Errno> {
            let mut total_written: Size = Size::new(0);
            for iovec in iovecs {
                let ptr: Pointer<u8> = iovec.buf();
                let len: Size = iovec.buf_len();

                let buf: Slice<'_, u8> = Slice::<u8>::for_raw_parts(memory, ptr, len);
                let buf: &[u8] = match buf.as_ref() {
                    Ok(slice) => slice,
                    Err(_) => {
                        ::nvx::log!("fd_write(): failed to get slice from memory");
                        return Err(Errno::Inval);
                    },
                };

                let written: Size = if !dry_run {
                    match file.write(buf) {
                        Ok(written) => written.into(),
                        Err(err) => {
                            ::nvx::log!("fd_write(): failed to write to file (errno={:?})", err);
                            return Err(Errno::from(err.value()));
                        },
                    }
                } else {
                    len
                };

                total_written += written;
            }
            Ok(total_written)
        };

        match self.get_file_mut(fd) {
            Some(file) => {
                // Ensure that we have the right to invoke this operation.
                if !file.rights_base().fd_write {
                    ::nvx::log!("fd_write(): access denied");
                    return Err(Errno::Acces);
                }

                let file: &mut File = file.file_mut();

                // Dry run to check for errors.
                write_iovecs(file, true)?;
                // Normal run to write to the file.
                write_iovecs(file, false)
            },
            None => {
                ::nvx::log!("fd_write(): invalid file descriptor");
                return Err(Errno::Badf);
            },
        }
    }
}
