// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(target_os = "linux")]
use crate::vmm::microvm::kvm::vmem::VirtualMemory;
#[cfg(target_os = "windows")]
use crate::vmm::microvm::whp::vmem::VirtualMemory;
use ::anyhow::Result;
use ::arch::mem::PAGE_SIZE;
use ::log::{
    error,
    trace,
};
#[cfg(target_os = "linux")]
use ::std::fs::Metadata;
use ::std::{
    fs::File,
    path::{
        Path,
        PathBuf,
    },
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
/// Encapsulates a RAM filesystem image and the operations required to load it into a guest's
/// physical memory.
///
/// On Linux, the image is file-backed (memory-mapped into guest memory via `remap_file_at`).
/// On Windows, the image is zero-copy file-backed via `MapViewOfFile3` with
/// `MEM_REPLACE_PLACEHOLDER`, mapping the file directly into the guest memory region.
///
#[derive(Debug)]
pub struct RamFs {
    /// Filesystem path from which the RAMFS image was loaded.
    path: PathBuf,
    /// Size of the RAMFS image in bytes.
    size: usize,
    /// Handle to the RAMFS image file.
    file: File,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl RamFs {
    ///
    /// # Description
    ///
    /// Opens a RAMFS image from the provided path and captures its metadata for later loading.
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

        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                let metadata: Metadata = match path.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        let reason: String = format!(
                            "failed retrieving ramfs metadata (path={path:?}, error={error:?})"
                        );
                        error!("RamFs::open(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                let size_u64: u64 = metadata.len();
                let size: usize = match usize::try_from(size_u64) {
                    Ok(size) => size,
                    Err(_error) => {
                        let reason: String = format!(
                            "ramfs image too large to fit on this platform (size={size_u64})"
                        );
                        error!("RamFs::open(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                let file: File = match File::open(path) {
                    Ok(file) => file,
                    Err(error) => {
                        let reason: String = format!(
                            "failed to open ramfs image (path={path:?}, error={error:?})"
                        );
                        error!("RamFs::open(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                Ok(Self {
                    path: path.to_path_buf(),
                    size,
                    file,
                })
            } else if #[cfg(target_os = "windows")] {
                let file: File = match File::open(path) {
                    Ok(file) => file,
                    Err(error) => {
                        let reason: String = format!(
                            "failed to open ramfs image (path={path:?}, error={error:?})"
                        );
                        error!("RamFs::open(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                let size_u64: u64 = match file.metadata() {
                    Ok(metadata) => metadata.len(),
                    Err(error) => {
                        let reason: String = format!(
                            "failed retrieving ramfs metadata (path={path:?}, error={error:?})"
                        );
                        error!("RamFs::open(): {reason}");
                        anyhow::bail!(reason)
                    },
                };

                let size: usize = match usize::try_from(size_u64) {
                    Ok(size) => size,
                    Err(_error) => {
                        let reason: String = format!(
                            "ramfs image too large to fit on this platform (size={size_u64})"
                        );
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
        }
    }

    /// Returns the size of the RAMFS image in bytes.
    fn ramfs_size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Loads this RAMFS image near the end of the provided virtual memory while maintaining the
    /// alignment and slack guarantees required by the initrd.
    ///
    /// On both Linux and Windows, the file is memory-mapped directly into guest memory
    /// (zero-copy).
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory that will host the RAMFS.
    /// - `initrd_end`: The first byte immediately after the initrd contents in guest memory.
    ///
    /// # Returns
    ///
    /// Upon success, returns the guest-physical base address and size where the RAMFS was loaded.
    /// Otherwise, returns an error.
    ///
    pub fn load_into_virtual_memory(
        &self,
        vmem: &mut VirtualMemory,
        initrd_end: usize,
    ) -> Result<(usize, usize)> {
        trace!(
            "RamFs::load_into_virtual_memory(): path={:?}, initrd_end={:#010x}",
            self.path, initrd_end
        );

        let ramfs_size: usize = self.ramfs_size();
        let memory_size: usize = vmem.get_size();

        if !ramfs_size.is_multiple_of(PAGE_SIZE) {
            let reason: String = format!(
                "ramfs image size is not page-aligned (ramfs_size={ramfs_size}, \
                 page_size={PAGE_SIZE})"
            );
            error!("RamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        if ramfs_size > memory_size {
            let reason: String = format!(
                "ramfs image exceeds guest memory size (ramfs_size={ramfs_size}, memory_size={})",
                memory_size
            );
            error!("RamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let min_available_base: usize = match initrd_end.checked_add(RAMFS_MIN_SLACK_BYTES) {
            Some(value) => value,
            None => {
                let reason: &str = "overflow while computing required ramfs slack";
                error!("RamFs::load_into_virtual_memory(): {reason}");
                anyhow::bail!(reason)
            },
        };

        if min_available_base > memory_size {
            let reason: String = format!(
                "guest memory ({memory_size}) is smaller than initrd end plus slack \
                 ({min_available_base})",
            );
            error!("RamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let ramfs_base: usize = match memory_size.checked_sub(ramfs_size) {
            Some(base) => base,
            None => {
                let reason: String = format!(
                    "ramfs image does not fit in guest memory (ramfs_size={ramfs_size}, \
                     memory_size={memory_size})",
                );
                error!("RamFs::load_into_virtual_memory(): {reason}");
                anyhow::bail!(reason)
            },
        };

        debug_assert!(
            ramfs_base.is_multiple_of(PAGE_SIZE),
            "ramfs_base ({ramfs_base:#x}) must be page-aligned"
        );

        if ramfs_base < min_available_base {
            let available: usize = match memory_size.checked_sub(min_available_base) {
                Some(value) => value,
                None => {
                    let reason: &str = "underflow while computing available guest memory for ramfs";
                    error!("RamFs::load_into_virtual_memory(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            let reason: String = format!(
                "ramfs image conflicts with initrd requirements (ramfs_size={ramfs_size}, \
                 available_for_ramfs={available}, required_slack={} bytes)",
                RAMFS_MIN_SLACK_BYTES,
            );
            error!("RamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        // Transfer RAMFS data into guest memory.
        self.map_file_into_guest(vmem, ramfs_base)?;

        trace!(
            "RamFs::load_into_virtual_memory(): loaded ramfs (path={:?}, base={:#010x}, \
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
    /// # Note
    ///
    /// The RAMFS registers at GPA `0xC` and GPA `0x10` fall inside the kernel ELF's `.zero`
    /// section (`LOAD` segment at GPA `0x0` with `MemSiz=0x8000`). The ELF loader zero-fills
    /// this range when `load_kernel()` runs. This method must therefore execute **after** the
    /// ELF has been loaded, so that the VMM-written values are not overwritten.
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
    /// Maps the RAMFS file directly into the provided guest memory region (zero-copy).
    ///
    /// On Linux, the file is remapped via `mmap(MAP_FIXED)`.
    /// On Windows, the file is mapped via `MapViewOfFile3` with `MEM_REPLACE_PLACEHOLDER`.
    ///
    fn map_file_into_guest(&self, vmem: &mut VirtualMemory, base: usize) -> Result<()> {
        trace!("RamFs::map_file_into_guest(): path={:?}, base={:#010x}", self.path, base);

        // Remap [base, base + file_size) of guest memory to be file-backed by the RAMFS image,
        // replacing the anonymous pages covering that range while leaving the rest untouched.
        vmem.remap_file_at(base, &self.file).map_err(|e| {
            let reason: String = format!(
                "failed to map ramfs image into guest memory (path={:?}, error={e})",
                self.path
            );
            error!("RamFs::map_file_into_guest(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        Ok(())
    }
}
