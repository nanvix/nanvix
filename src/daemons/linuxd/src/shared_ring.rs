// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::nvx_ring::{
    HOST_CQ_SIGNAL_OFFSET,
    HOST_SQ_SIGNAL_OFFSET,
    REGION_SIZE,
};
use ::std::{
    fs,
    fs::File,
    os::unix::io::AsRawFd,
    path::{
        Path,
        PathBuf,
    },
};
use ::user_vm_api::IVSHMEM_RING_TRANSPORT_LOCATOR_AUTO;

const IVSHMEM_SYSFS_ROOT: &str = "/sys/bus/pci/devices";
const IVSHMEM_VENDOR_ID: &str = "0x1af4";
const IVSHMEM_DEVICE_ID: &str = "0x1110";
const IVSHMEM_BAR_INDEX: usize = 2;
// Must stay in sync with `config::microvm::RING_BUFFER_GPA`, but linuxd's shared-ring helpers also
// compile in configs where the `config` crate's `microvm` feature is disabled.
const RING_BUFFER_GPA: u64 = 0x0100_0000;

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
            .map_err(|e| {
                anyhow::anyhow!("failed to open shared ring backing file {:?}: {e}", path)
            })?;

        let file_len: u64 = file
            .metadata()
            .map_err(|e| {
                anyhow::anyhow!("failed to stat shared ring backing file {:?}: {e}", path)
            })?
            .len();
        let required: u64 = REGION_SIZE as u64;
        if file_len < required {
            let reason: String = format!(
                "shared ring backing file too small: expected at least {required} bytes, got \
                 {file_len}"
            );
            error!("SharedRing::open(): {reason}");
            anyhow::bail!(reason)
        }

        Self::mmap(file, &format!("{path:?}"))
    }

    pub fn open_ivshmem(locator: &str) -> Result<Self> {
        let resource_path: PathBuf = Self::resolve_ivshmem_resource(locator)?;
        trace!(
            "SharedRing::open_ivshmem(): locator={locator}, resource_path={}",
            resource_path.display()
        );

        let file: File = File::options()
            .read(true)
            .write(true)
            .open(&resource_path)
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to open ivshmem BAR{} resource {}: {e}",
                    IVSHMEM_BAR_INDEX,
                    resource_path.display()
                )
            })?;

        Self::mmap(file, &resource_path.display().to_string())
    }

    fn resolve_ivshmem_resource(locator: &str) -> Result<PathBuf> {
        if locator == IVSHMEM_RING_TRANSPORT_LOCATOR_AUTO {
            return Self::find_ivshmem_resource();
        }

        let Some(pci_address) = locator.strip_prefix("pci=") else {
            anyhow::bail!(
                "unsupported ivshmem locator {locator:?}; expected \
                 {IVSHMEM_RING_TRANSPORT_LOCATOR_AUTO:?} or pci=<BDF>"
            );
        };

        Ok(Path::new(IVSHMEM_SYSFS_ROOT)
            .join(pci_address)
            .join(format!("resource{IVSHMEM_BAR_INDEX}")))
    }

    fn find_ivshmem_resource() -> Result<PathBuf> {
        let mut matches: Vec<PathBuf> = Vec::new();
        for entry in fs::read_dir(IVSHMEM_SYSFS_ROOT)
            .map_err(|e| anyhow::anyhow!("failed to read {IVSHMEM_SYSFS_ROOT}: {e}"))?
        {
            let entry =
                entry.map_err(|e| anyhow::anyhow!("failed to enumerate PCI devices: {e}"))?;
            let device_path: PathBuf = entry.path();

            let vendor: String = match fs::read_to_string(device_path.join("vendor")) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let device: String = match fs::read_to_string(device_path.join("device")) {
                Ok(value) => value,
                Err(_) => continue,
            };

            if vendor.trim() == IVSHMEM_VENDOR_ID && device.trim() == IVSHMEM_DEVICE_ID {
                let resource_path: PathBuf =
                    device_path.join(format!("resource{IVSHMEM_BAR_INDEX}"));
                if resource_path.exists() {
                    matches.push(resource_path);
                }
            }
        }

        match matches.len() {
            0 => anyhow::bail!(
                "failed to auto-discover ivshmem BAR{} resource under {}",
                IVSHMEM_BAR_INDEX,
                IVSHMEM_SYSFS_ROOT
            ),
            1 => Ok(matches.remove(0)),
            count => anyhow::bail!(
                "ivshmem auto-discovery is ambiguous: found {count} matching BAR{} resources",
                IVSHMEM_BAR_INDEX
            ),
        }
    }

    fn mmap(file: File, source: &str) -> Result<Self> {
        let file_len: u64 = file
            .metadata()
            .ok()
            .map(|metadata| metadata.len())
            .unwrap_or_default();

        let mapped_ptr: *mut u8 = unsafe {
            ::libc::mmap(
                ::std::ptr::null_mut(),
                REGION_SIZE,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                ::libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
            .cast::<u8>()
        };

        if mapped_ptr.is_null() || mapped_ptr == ::libc::MAP_FAILED.cast::<u8>() {
            let reason: String = format!(
                "failed to mmap shared ring source {} (metadata_len={}): {}",
                source,
                file_len,
                ::std::io::Error::last_os_error()
            );
            error!("SharedRing::mmap(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(Self {
            ptr: mapped_ptr,
            len: REGION_SIZE,
        })
    }

    pub fn ring_ptr_from_gpa(&self, gpa: u64, len: usize) -> Result<*mut u8> {
        let region_len: u64 = REGION_SIZE as u64;
        if gpa < RING_BUFFER_GPA {
            let reason: String =
                format!("ring GPA below base (gpa={gpa:#x}, base={RING_BUFFER_GPA:#x})");
            error!("SharedRing::ring_ptr_from_gpa(): {reason}");
            anyhow::bail!(reason)
        }

        let offset_u64: u64 = gpa - RING_BUFFER_GPA;
        let end_u64: u64 = offset_u64
            .checked_add(u64::try_from(len).map_err(|e| anyhow::anyhow!("length too large: {e}"))?)
            .ok_or_else(|| anyhow::anyhow!("ring GPA range overflow"))?;
        if end_u64 > region_len {
            let reason: String = format!(
                "ring GPA range exceeds mapping (gpa={gpa:#x}, len={len}, region_len={region_len})"
            );
            error!("SharedRing::ring_ptr_from_gpa(): {reason}");
            anyhow::bail!(reason)
        }

        let offset: usize = usize::try_from(offset_u64)
            .map_err(|e| anyhow::anyhow!("ring GPA offset does not fit usize: {e}"))?;
        Ok(
            // SAFETY: `offset` and `len` were bounds-checked above.
            unsafe { self.ptr.add(offset) },
        )
    }

    pub fn write_bytes(&self, offset: usize, data: &[u8]) -> Result<()> {
        let end: usize = offset
            .checked_add(data.len())
            .ok_or_else(|| anyhow::anyhow!("offset overflow"))?;
        if end > self.len {
            let reason: String = format!(
                "write exceeds mapping (offset={offset}, len={}, mapping_len={})",
                data.len(),
                self.len
            );
            error!("SharedRing::write_bytes(): {reason}");
            anyhow::bail!(reason)
        }

        // SAFETY: bounds were checked above and the regions do not overlap.
        unsafe { ::std::ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(offset), data.len()) };
        Ok(())
    }

    pub fn read_copy<T: Copy>(&self, offset: usize) -> Result<T> {
        let end: usize = offset
            .checked_add(core::mem::size_of::<T>())
            .ok_or_else(|| anyhow::anyhow!("offset overflow"))?;
        if end > self.len {
            let reason: String = format!(
                "typed read exceeds mapping (offset={offset}, size={}, mapping_len={})",
                core::mem::size_of::<T>(),
                self.len
            );
            error!("SharedRing::read_copy(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(
            // SAFETY: bounds were checked above and `T: Copy` is plain-data for our call sites.
            unsafe { core::ptr::read_unaligned(self.ptr.add(offset).cast::<T>()) },
        )
    }

    pub fn write_copy<T: Copy>(&self, offset: usize, value: T) -> Result<()> {
        let end: usize = offset
            .checked_add(core::mem::size_of::<T>())
            .ok_or_else(|| anyhow::anyhow!("offset overflow"))?;
        if end > self.len {
            let reason: String = format!(
                "typed write exceeds mapping (offset={offset}, size={}, mapping_len={})",
                core::mem::size_of::<T>(),
                self.len
            );
            error!("SharedRing::write_copy(): {reason}");
            anyhow::bail!(reason)
        }

        // SAFETY: bounds were checked above and `T: Copy` is plain-data for our call sites.
        unsafe { core::ptr::write_unaligned(self.ptr.add(offset).cast::<T>(), value) };
        Ok(())
    }

    pub fn sq_signal_word(&self) -> *mut u32 {
        // SAFETY: the notification word is inside the mapped control area.
        unsafe { self.ptr.add(HOST_SQ_SIGNAL_OFFSET).cast::<u32>() }
    }

    pub fn cq_signal_word(&self) -> *mut u32 {
        // SAFETY: the notification word is inside the mapped control area.
        unsafe { self.ptr.add(HOST_CQ_SIGNAL_OFFSET).cast::<u32>() }
    }

    pub fn fixed_buffer_ptr(&self, buffer_id: u32) -> Result<*mut u8> {
        let buffer_index: usize = usize::try_from(buffer_id)
            .map_err(|e| anyhow::anyhow!("invalid fixed buffer id {buffer_id}: {e}"))?;
        if buffer_index >= ::nvx_ring::FIXED_BUF_COUNT {
            let reason: String = format!("fixed buffer id out of range ({buffer_id})");
            error!("SharedRing::fixed_buffer_ptr(): {reason}");
            anyhow::bail!(reason)
        }

        let offset: usize =
            ::nvx_ring::FIXED_BUF_OFFSET + buffer_index * ::nvx_ring::FIXED_BUF_SIZE;
        let end: usize = offset + ::nvx_ring::FIXED_BUF_SIZE;
        if end > self.len {
            let reason: String = format!(
                "fixed buffer range exceeds mapping (buffer_id={buffer_id}, offset={offset}, \
                 end={end}, len={})",
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
