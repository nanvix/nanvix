// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::ptr;

//==================================================================================================
// Structures
//==================================================================================================

pub struct FileMapping {
    fd: ::libc::c_int,
    ptr: *mut ::libc::c_void,
    size: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl FileMapping {
    /// Maps a file into memory.
    pub fn mmap(filename: &str) -> Result<Self> {
        trace!("mmap(): filename={}", filename);

        // Open the file.
        let fd: i32 = unsafe {
            let filename: std::ffi::CString = ::std::ffi::CString::new(filename)?;
            let filename: &[u8] = filename.as_bytes_with_nul();
            ::libc::open(filename.as_ptr() as *const ::libc::c_char, ::libc::O_RDONLY)
        };

        if fd < 0 {
            anyhow::bail!("failed to open file");
        }

        // Get file size.
        let size: usize = unsafe {
            let mut stat: ::libc::stat = ::std::mem::zeroed();
            if ::libc::fstat(fd, &mut stat) < 0 {
                if ::libc::close(fd) < 0 {
                    warn!("failed to close file");
                }
                anyhow::bail!("failed to get file size");
            }
            stat.st_size as usize
        };

        // Map the file.
        let ptr: *mut std::ffi::c_void = unsafe {
            ::libc::mmap(ptr::null_mut(), size, ::libc::PROT_READ, ::libc::MAP_PRIVATE, fd, 0)
        };

        if ptr == ::libc::MAP_FAILED {
            unsafe {
                if ::libc::close(fd) < 0 {
                    warn!("failed to close file");
                }
            }
            anyhow::bail!("failed to map file");
        }

        Ok(Self { fd, size, ptr })
    }

    pub fn ptr(&self) -> *const u8 {
        self.ptr as *const u8
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for FileMapping {
    fn drop(&mut self) {
        unsafe {
            if ::libc::munmap(self.ptr, self.size) < 0 {
                warn!("failed to unmap file");
            }
            if ::libc::close(self.fd) < 0 {
                warn!("failed to close file");
            }
        }
    }
}
