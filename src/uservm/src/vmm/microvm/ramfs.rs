// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::kvm::vmem::VirtualMemory;
use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::libc::c_int;
use ::std::{
    fs::{
        File,
        Metadata,
    },
    os::fd::AsRawFd,
    path::{
        Path,
        PathBuf,
    },
};
use ::syslog::{
    error,
    trace,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Minimum slack size (in bytes) required between the end of the initrd and the start of the RAMFS.
const RAMFS_MIN_SLACK_BYTES: usize = 4 * 1024 * 1024;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Encapsulates a RAM filesystem image, its metadata, and the operations required to map it into a
/// guest's physical memory.
///
#[derive(Debug)]
pub struct RamFs {
    /// Filesystem path from which the RAMFS image was loaded.
    path: PathBuf,
    /// Size of the RAMFS image in bytes.
    size: usize,
    /// Handle to the RAMFS image file used for memory-mapping.
    file: File,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RamFs {
    ///
    /// # Description
    ///
    /// Opens a RAMFS image from the provided path and captures its metadata for later mapping.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the RAMFS image on the host filesystem.
    ///
    /// # Returns
    ///
    /// Upon success, returns a `RamFs` descriptor. Otherwise, returns an error.
    ///
    pub fn open(path: &Path) -> Result<Self> {
        trace!("RamFs::open(): path={path:?}");

        let metadata: Metadata = match path.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                let reason: String =
                    format!("failed retrieving ramfs metadata (path={path:?}, error={error:?})");
                error!("RamFs::open(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let size_u64: u64 = metadata.len();
        let size: usize = match usize::try_from(size_u64) {
            Ok(size) => size,
            Err(_error) => {
                let reason: String =
                    format!("ramfs image too large to fit on this platform (size={size_u64})");
                error!("RamFs::open(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let file: File = match File::open(path) {
            Ok(file) => file,
            Err(error) => {
                let reason: String =
                    format!("failed to open ramfs image (path={path:?}, error={error:?})");
                error!("RamFs::open(): {reason}");
                anyhow::bail!(reason)
            },
        };

        Ok(Self {
            path: path.to_path_buf(),
            size,
            file,
        })
    }

    ///
    /// # Description
    ///
    /// Maps this RAMFS image near the end of the provided virtual memory while maintaining the
    /// alignment and slack guarantees required by the initrd.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory that will host the RAMFS.
    /// - `initrd_end`: The first byte immediately after the initrd contents in guest memory.
    ///
    /// # Returns
    ///
    /// Upon success, returns the guest-physical base address and size where the RAMFS was mapped.
    /// Otherwise, returns an error.
    ///
    pub fn map_into_virtual_memory(
        &self,
        vmem: &mut VirtualMemory,
        initrd_end: usize,
    ) -> Result<(usize, usize)> {
        trace!(
            "RamFs::map_into_virtual_memory(): path={:?}, initrd_end={:#010x}",
            self.path, initrd_end
        );

        let ramfs_size: usize = self.size;
        let memory_size: usize = vmem.get_size();

        if ramfs_size > memory_size {
            let reason: String = format!(
                "ramfs image exceeds guest memory size (ramfs_size={ramfs_size}, memory_size={})",
                memory_size
            );
            error!("RamFs::map_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let min_available_base: usize = match initrd_end.checked_add(RAMFS_MIN_SLACK_BYTES) {
            Some(value) => value,
            None => {
                let reason: &str = "overflow while computing required ramfs slack";
                error!("RamFs::map_into_virtual_memory(): {reason}");
                anyhow::bail!(reason)
            },
        };

        if min_available_base > memory_size {
            let reason: String = format!(
                "guest memory ({memory_size}) is smaller than initrd end plus slack \
                 ({min_available_base})",
            );
            error!("RamFs::map_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let ramfs_base_unaligned: usize = match memory_size.checked_sub(ramfs_size) {
            Some(base) => base,
            None => {
                let reason: String = format!(
                    "ramfs image does not fit in guest memory (ramfs_size={ramfs_size}, \
                     memory_size={memory_size})",
                );
                error!("RamFs::map_into_virtual_memory(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let ramfs_base: usize = ramfs_base_unaligned - (ramfs_base_unaligned % PAGE_SIZE);

        if ramfs_base < min_available_base {
            let available: usize = match memory_size.checked_sub(min_available_base) {
                Some(value) => value,
                None => {
                    let reason: &str = "underflow while computing available guest memory for ramfs";
                    error!("RamFs::map_into_virtual_memory(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            let reason: String = format!(
                "ramfs image conflicts with initrd requirements (ramfs_size={ramfs_size}, \
                 available_for_ramfs={available}, required_slack={} bytes)",
                RAMFS_MIN_SLACK_BYTES,
            );
            error!("RamFs::map_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let guest_ptr: *mut u8 = vmem.get_raw_ptr();
        self.map_file_into_guest(guest_ptr, ramfs_base, ramfs_size)?;

        trace!(
            "RamFs::map_into_virtual_memory(): mapped ramfs (path={:?}, base={:#010x}, \
             size={ramfs_size})",
            self.path, ramfs_base
        );

        Ok((ramfs_base, ramfs_size))
    }

    ///
    /// # Description
    ///
    /// Writes the RAMFS base and size registers exposed to the guest.
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory instance that holds the control registers.
    /// - `ramfs_region`: Optional tuple containing the guest-physical base and size of the RAMFS.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_registers(
        vmem: &mut VirtualMemory,
        ramfs_region: Option<(usize, usize)>,
    ) -> Result<()> {
        let base_register_addr: u64 = ::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_BASE as u64;
        let size_register_addr: u64 = ::config::microvm::DEFAULT_MICROVM_CTRL_RAMFS_SIZE as u64;

        let (base_value, size_value): (u32, u32) = match ramfs_region {
            Some((base, size)) => {
                trace!("RamFs::write_registers(): base={:#010x}, size={:#x}", base, size);

                let base_value: u32 = match u32::try_from(base) {
                    Ok(value) => value,
                    Err(_) => {
                        let reason: String =
                            format!("ramfs base does not fit into 32 bits (base={base:#010x})");
                        error!("RamFs::write_registers(): {reason}");
                        anyhow::bail!(reason)
                    },
                };
                let size_value: u32 = match u32::try_from(size) {
                    Ok(value) => value,
                    Err(_) => {
                        let reason: String =
                            format!("ramfs size does not fit into 32 bits (size={size:#010x})");
                        error!("RamFs::write_registers(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                (base_value, size_value)
            },
            None => (0, 0),
        };

        let base_bytes: [u8; 4] = base_value.to_le_bytes();
        let size_bytes: [u8; 4] = size_value.to_le_bytes();

        vmem.write_bytes(base_register_addr, &base_bytes)?;
        vmem.write_bytes(size_register_addr, &size_bytes)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Maps the RAMFS file directly into the provided guest memory region.
    ///
    /// # Parameters
    ///
    /// - `guest_ptr`: Base pointer of the guest memory.
    /// - `base`: Guest-physical base address where the RAMFS should reside.
    /// - `length`: Number of bytes to map.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    fn map_file_into_guest(&self, guest_ptr: *mut u8, base: usize, length: usize) -> Result<()> {
        trace!(
            "RamFs::map_file_into_guest(): path={:?}, base={:#010x}, length={length}",
            self.path, base
        );

        if length > self.size {
            let reason: String = format!(
                "requested ramfs mapping larger than file (requested={length}, size={})",
                self.size
            );
            error!("RamFs::map_file_into_guest(): {reason}");
            anyhow::bail!(reason)
        }

        // Mapping with MAP_FIXED replaces the anonymous pages covering [base, base + length), while
        // leaving the remaining guest memory untouched, effectively swapping the backing store for
        // that specific slice.
        // SAFETY: `base` has been bounds-checked and page-aligned, so adding it to `guest_ptr`
        // stays within the allocated KVM userspace memory region owned by `vmem`.
        let dst: *mut ::libc::c_void = unsafe { guest_ptr.add(base).cast::<::libc::c_void>() };
        // SAFETY: `dst` points to a mapped, page-aligned region we own, `length` fits within that
        // region, and `self.file` remains open for the lifetime of the mapping, making the `mmap`
        // call well-defined.
        let mapped_ptr: *mut ::libc::c_void = unsafe {
            ::libc::mmap(
                dst,
                length,
                ::libc::PROT_READ | ::libc::PROT_WRITE,
                ::libc::MAP_PRIVATE | ::libc::MAP_FIXED,
                self.file.as_raw_fd(),
                0,
            )
        };

        if mapped_ptr.is_null() || mapped_ptr == ::libc::MAP_FAILED || mapped_ptr != dst {
            // SAFETY: `__errno_location()` returns a valid thread-local pointer for the current
            // thread, so dereferencing it immediately after the syscall is sound.
            let errno: c_int = unsafe { *::libc::__errno_location() };
            let reason: String = format!(
                "failed to map ramfs image into guest memory (path={:?}, errno={errno})",
                self.path
            );
            error!("RamFs::map_file_into_guest(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(())
    }
}
