// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::std::{
    fs::File,
    os::unix::io::AsRawFd,
    path::Path,
};

/// Shared mapping of a user VM ring-buffer backing file.
pub struct SharedRing {
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for SharedRing {}
unsafe impl Sync for SharedRing {}

impl SharedRing {
    pub fn open(path: &Path) -> Result<Self> {
        trace!("SharedRing::open(): path={:?}", path);

        let file: File = File::options()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| anyhow::anyhow!("failed to open shared ring backing file {:?}: {e}", path))?;

        let file_len: u64 = file
            .metadata()
            .map_err(|e| anyhow::anyhow!("failed to stat shared ring backing file {:?}: {e}", path))?
            .len();
        let required: u64 = ::nvx_ring::REGION_SIZE as u64;
        if file_len < required {
            let reason: String = format!(
                "shared ring backing file too small: expected at least {required} bytes, got {file_len}"
            );
            error!("SharedRing::open(): {reason}");
            anyhow::bail!(reason)
        }

        let mapped_ptr: *mut u8 = unsafe {
            ::libc::mmap(
                ::std::ptr::null_mut(),
                ::nvx_ring::REGION_SIZE,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                ::libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
            .cast::<u8>()
        };

        if mapped_ptr.is_null() || mapped_ptr == ::libc::MAP_FAILED.cast::<u8>() {
            let reason: String = format!(
                "failed to mmap shared ring backing file {:?}: {}",
                path,
                ::std::io::Error::last_os_error()
            );
            error!("SharedRing::open(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(Self {
            ptr: mapped_ptr,
            len: ::nvx_ring::REGION_SIZE,
        })
    }

    pub fn fixed_buffer_ptr(&self, buffer_id: u32) -> Result<*mut u8> {
        let buffer_index: usize = usize::try_from(buffer_id)
            .map_err(|e| anyhow::anyhow!("invalid fixed buffer id {buffer_id}: {e}"))?;
        if buffer_index >= ::nvx_ring::FIXED_BUF_COUNT {
            let reason: String = format!("fixed buffer id out of range ({buffer_id})");
            error!("SharedRing::fixed_buffer_ptr(): {reason}");
            anyhow::bail!(reason)
        }

        let offset: usize = ::nvx_ring::FIXED_BUF_OFFSET + buffer_index * ::nvx_ring::FIXED_BUF_SIZE;
        let end: usize = offset + ::nvx_ring::FIXED_BUF_SIZE;
        if end > self.len {
            let reason: String = format!(
                "fixed buffer range exceeds mapping (buffer_id={buffer_id}, offset={offset}, end={end}, len={})",
                self.len
            );
            error!("SharedRing::fixed_buffer_ptr(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(
            // SAFETY: `offset` and `end` were bounds-checked against the mapping length.
            unsafe { self.ptr.add(offset) },
        )
    }
}

impl Drop for SharedRing {
    fn drop(&mut self) {
        let ret: i32 = unsafe { ::libc::munmap(self.ptr.cast::<::libc::c_void>(), self.len) };
        if ret != 0 {
            error!("SharedRing::drop(): munmap failed (ret={ret})");
        }
    }
}
