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
    info,
    trace,
    warn,
};
use ::multiimage::MultiImageLayout;
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
        extra_remap_regions: &[(usize, &File)],
    ) -> Result<(usize, usize)> {
        trace!(
            "RamFs::load_into_virtual_memory(): path={:?}, initrd_end={:#010x}, extra_regions={}",
            self.path,
            initrd_end,
            extra_remap_regions.len()
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
        if extra_remap_regions.is_empty() {
            // Standard path: single-file remap for just the ramfs.
            self.map_file_into_guest(vmem, ramfs_base)?;
        } else {
            // Combined zero-copy path: remap extra files (e.g. initrd) + ramfs together.
            let mut regions: Vec<(usize, &File)> = extra_remap_regions.to_vec();
            regions.push((ramfs_base, &self.file));
            vmem.remap_files_at(&regions)?;

            // Prefault ramfs pages after the combined remap.
            cfg_if::cfg_if! {
                if #[cfg(target_os = "linux")] {
                    vmem.madvise_at(ramfs_base, ramfs_size, ::libc::MADV_POPULATE_READ)
                        .unwrap_or_else(|e| {
                            warn!(
                                "RamFs::load_into_virtual_memory(): MADV_POPULATE_READ failed \
                                 ({e}), falling back to MADV_WILLNEED"
                            );
                            vmem.madvise_at(ramfs_base, ramfs_size, ::libc::MADV_WILLNEED)
                                .unwrap_or_else(|e2| {
                                    warn!(
                                        "RamFs::load_into_virtual_memory(): MADV_WILLNEED \
                                         fallback also failed: {e2}"
                                    );
                                });
                        });
                } else if #[cfg(target_os = "windows")] {
                    vmem.prefault_at(ramfs_base, ramfs_size)
                        .unwrap_or_else(|e| {
                            warn!(
                                "RamFs::load_into_virtual_memory(): prefault_at failed: {e}"
                            );
                        });
                }
            }
        }

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
    /// section (`LOAD` segment at GPA `0x0` with `MemSiz=0x8000`), which `load_kernel()`
    /// zero-fills by default. This method must therefore execute **after** the ELF has been
    /// loaded, so that the VMM-written values are not overwritten. (With the
    /// `nightly-performance-optimizations` feature the loader skips that zeroing and relies on
    /// the freshly allocated guest memory already being zero, but running after `load_kernel()`
    /// remains correct.)
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

        // Advise the host OS to prefault ramfs pages, reducing page-fault stalls
        // during guest execution.
        cfg_if::cfg_if! {
            if #[cfg(target_os = "linux")] {
                // MADV_POPULATE_READ synchronously faults in file-backed pages as read-only,
                // which avoids copy-on-write on MAP_PRIVATE mappings. This is strictly stronger
                // than MADV_WILLNEED (advisory only) and ensures all pages are host-resident
                // before the guest starts, so KVM EPT faults resolve without blocking on I/O.
                // Available since Linux 5.14; fall back to MADV_WILLNEED on older kernels.
                vmem.madvise_at(base, self.ramfs_size(), ::libc::MADV_POPULATE_READ)
                    .unwrap_or_else(|e| {
                        warn!(
                            "RamFs::map_file_into_guest(): MADV_POPULATE_READ failed ({e}), \
                             falling back to MADV_WILLNEED"
                        );
                        vmem.madvise_at(base, self.ramfs_size(), ::libc::MADV_WILLNEED)
                            .unwrap_or_else(|e2| {
                                warn!(
                                    "RamFs::map_file_into_guest(): MADV_WILLNEED fallback \
                                     also failed: {e2}"
                                );
                            });
                    });
            } else if #[cfg(target_os = "windows")] {
                // PrefetchVirtualMemory is the Windows equivalent of MADV_WILLNEED:
                // it brings backing pages into physical memory ahead of time.
                vmem.prefault_at(base, self.ramfs_size())
                    .unwrap_or_else(|e| {
                        warn!("RamFs::map_file_into_guest(): prefault_at failed: {e}");
                    });
            }
        }

        Ok(())
    }
}

//==================================================================================================
// Multi-Image RAMFS
//==================================================================================================

///
/// # Description
///
/// Encapsulates a multi-image RAMFS layout comprising a HEAD page and one or more sub-image files.
///
/// Instead of creating a single concatenated file on disk, this struct holds the pre-built HEAD
/// page and open file handles for each sub-image. The VMM backends map each file directly into
/// guest memory (zero-copy) at the offsets computed by the multi-image layout.
///
#[derive(Debug)]
pub struct MultiRamFs {
    /// Pre-built HEAD page bytes (header + entries).
    head_page: std::vec::Vec<u8>,
    /// Open file handles paired with their guest memory offset (relative to ramfs_base).
    files: std::vec::Vec<(File, usize)>,
    /// Total size of the multi-image container (HEAD + all page-aligned sub-images).
    total_size: usize,
}

impl MultiRamFs {
    ///
    /// # Description
    ///
    /// Opens all sub-image files described by the layout and prepares them for zero-copy
    /// mapping into guest memory.
    ///
    /// # Parameters
    ///
    /// - `layout`: The multi-image layout produced by [`multiimage::compute_multiimage_layout`].
    ///
    /// # Returns
    ///
    /// Upon success, returns a `MultiRamFs` descriptor. Otherwise, returns an error.
    ///
    pub fn open(layout: MultiImageLayout) -> Result<Self> {
        trace!(
            "MultiRamFs::open(): {} regions, total_size={}",
            layout.regions.len(),
            layout.total_size
        );

        let mut files: std::vec::Vec<(File, usize)> =
            std::vec::Vec::with_capacity(layout.regions.len());

        for region in &layout.regions {
            let file: File = File::open(&region.path).map_err(|e| {
                let reason: String =
                    format!("failed to open sub-image (path={:?}, error={e:?})", region.path);
                error!("MultiRamFs::open(): {reason}");
                anyhow::anyhow!(reason)
            })?;

            // Validate that the file size matches what the layout expects.
            let actual_size: u64 = file
                .metadata()
                .map_err(|e| {
                    let reason: String = format!(
                        "failed to query sub-image metadata (path={:?}, error={e:?})",
                        region.path
                    );
                    error!("MultiRamFs::open(): {reason}");
                    anyhow::anyhow!(reason)
                })?
                .len();

            if actual_size != region.size as u64 {
                let reason: String = format!(
                    "sub-image size changed since layout was computed (path={:?}, expected={}, \
                     actual={actual_size})",
                    region.path, region.size
                );
                error!("MultiRamFs::open(): {reason}");
                anyhow::bail!(reason);
            }

            files.push((file, region.offset));
        }

        Ok(Self {
            head_page: layout.head_page,
            files,
            total_size: layout.total_size,
        })
    }

    /// Returns the total page-aligned size of the multi-image container.
    fn total_size(&self) -> usize {
        self.total_size
    }

    ///
    /// # Description
    ///
    /// Loads this multi-image RAMFS near the end of guest memory, writing the HEAD page and
    /// memory-mapping each sub-image file at its computed offset (zero-copy).
    ///
    /// # Parameters
    ///
    /// - `vmem`: Virtual memory that will host the RAMFS.
    /// - `initrd_end`: The first byte immediately after the initrd contents in guest memory.
    ///
    /// # Returns
    ///
    /// Upon success, returns the guest-physical base address and total size where the RAMFS
    /// was loaded. Otherwise, returns an error.
    ///
    pub fn load_into_virtual_memory(
        &self,
        vmem: &mut VirtualMemory,
        initrd_end: usize,
        extra_remap_regions: &[(usize, &File)],
    ) -> Result<(usize, usize)> {
        trace!(
            "MultiRamFs::load_into_virtual_memory(): total_size={}, initrd_end={:#010x}, \
             extra_regions={}",
            self.total_size,
            initrd_end,
            extra_remap_regions.len()
        );

        let ramfs_size: usize = self.total_size();
        let memory_size: usize = vmem.get_size();

        if !ramfs_size.is_multiple_of(PAGE_SIZE) {
            let reason: String = format!(
                "multi-image total size is not page-aligned (total_size={ramfs_size}, \
                 page_size={PAGE_SIZE})"
            );
            error!("MultiRamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        if ramfs_size > memory_size {
            let reason: String = format!(
                "multi-image exceeds guest memory size (total_size={ramfs_size}, \
                 memory_size={memory_size})"
            );
            error!("MultiRamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let min_available_base: usize = match initrd_end.checked_add(RAMFS_MIN_SLACK_BYTES) {
            Some(value) => value,
            None => {
                let reason: &str = "overflow while computing required ramfs slack";
                error!("MultiRamFs::load_into_virtual_memory(): {reason}");
                anyhow::bail!(reason)
            },
        };

        if min_available_base > memory_size {
            let reason: String = format!(
                "guest memory ({memory_size}) is smaller than initrd end plus slack \
                 ({min_available_base})",
            );
            error!("MultiRamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        let ramfs_base: usize = match memory_size.checked_sub(ramfs_size) {
            Some(base) => base,
            None => {
                let reason: String = format!(
                    "multi-image does not fit in guest memory (total_size={ramfs_size}, \
                     memory_size={memory_size})",
                );
                error!("MultiRamFs::load_into_virtual_memory(): {reason}");
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
                    error!("MultiRamFs::load_into_virtual_memory(): {reason}");
                    anyhow::bail!(reason)
                },
            };
            let reason: String = format!(
                "multi-image conflicts with initrd requirements (total_size={ramfs_size}, \
                 available_for_ramfs={available}, required_slack={} bytes)",
                RAMFS_MIN_SLACK_BYTES,
            );
            error!("MultiRamFs::load_into_virtual_memory(): {reason}");
            anyhow::bail!(reason)
        }

        // Build the list of file-backed regions, prepending any extra remap regions
        // (e.g. initrd) so they are included in a single combined remap.
        let mut regions: std::vec::Vec<(usize, &File)> = extra_remap_regions.to_vec();
        regions.extend(
            self.files
                .iter()
                .map(|(file, offset)| (ramfs_base + offset, file)),
        );

        // When extra regions are present, the HEAD page at ramfs_base falls inside the
        // freed placeholder range and is destroyed by remap_files_at. Write it BEFORE
        // the remap only when there are no extra regions (the head [0..split_start) is
        // preserved); otherwise write it AFTER, into the re-committed gap.
        if extra_remap_regions.is_empty() {
            vmem.write_bytes(ramfs_base as u64, &self.head_page)?;
            info!(
                "MultiRamFs::load_into_virtual_memory(): wrote HEAD page at {:#010x} (before \
                 remap)",
                ramfs_base
            );
        }

        vmem.remap_files_at(&regions)?;

        if !extra_remap_regions.is_empty() {
            vmem.write_bytes(ramfs_base as u64, &self.head_page)?;
            info!(
                "MultiRamFs::load_into_virtual_memory(): wrote HEAD page at {:#010x} (after remap)",
                ramfs_base
            );
        }

        for &(guest_addr, _) in &regions {
            trace!(
                "MultiRamFs::load_into_virtual_memory(): mapped sub-image at {:#010x}",
                guest_addr
            );
        }

        info!(
            "MultiRamFs::load_into_virtual_memory(): loaded multi-image (base={:#010x}, \
             size={ramfs_size})",
            ramfs_base
        );

        Ok((ramfs_base, ramfs_size))
    }

    ///
    /// # Description
    ///
    /// Extracts the open file handles from this `MultiRamFs`, consuming it.
    ///
    /// The returned files must be kept alive for the VM's lifetime so that the memory-mapped
    /// regions remain valid.
    ///
    pub fn into_files(self) -> std::vec::Vec<File> {
        self.files.into_iter().map(|(file, _offset)| file).collect()
    }
}

//==================================================================================================
// Shared RAMFS Loading
//==================================================================================================

/// Result of [`load_ramfs`]: describes which RAMFS variant was loaded into guest memory so
/// that the caller can keep the appropriate file handles alive for the VM's lifetime.
pub enum LoadedRamFs {
    /// No RAMFS was loaded (`-ramfs` was not specified).
    None,
    /// A single-image RAMFS was loaded via the legacy path.
    Single {
        /// The opened RAMFS descriptor (keeps the file handle alive).
        ramfs: RamFs,
        /// Guest-physical base address of the RAMFS.
        base: usize,
        /// Size in bytes.
        size: usize,
    },
}

impl LoadedRamFs {
    /// Returns the guest-physical region `(base, size)` if any RAMFS was loaded.
    pub fn region(&self) -> Option<(usize, usize)> {
        match self {
            Self::None => None,
            Self::Single { base, size, .. } => Some((*base, *size)),
        }
    }
}

///
/// # Description
///
/// Loads a RAMFS image into guest memory. This shared helper eliminates duplication between the
/// KVM and WHP backends.
///
/// When `ramfs_filename` is `Some`, opens the image and maps it into guest memory.
/// When `None`, returns [`LoadedRamFs::None`].
///
/// The caller is responsible for keeping the returned file handles alive for the VM's lifetime
/// (e.g., via `VirtualMemory::attach_ramfs`).
///
/// # Parameters
///
/// - `vmem`: Guest virtual memory to load the RAMFS into.
/// - `initrd_end`: First byte after the initrd in guest memory.
/// - `ramfs_filename`: Optional path to a single RAMFS image.
/// - `extra_remap_regions`: Additional file-backed regions (e.g. initrd) to include in the
///   combined zero-copy remap. Pass `&[]` when no extra regions are needed.
///
/// # Returns
///
/// On success, returns a [`LoadedRamFs`] describing what was loaded and any file handles that
/// must be kept alive. On failure, returns an error.
///
pub fn load_ramfs(
    vmem: &mut VirtualMemory,
    initrd_end: usize,
    ramfs_filename: Option<&str>,
    extra_remap_regions: &[(usize, &File)],
) -> Result<LoadedRamFs> {
    if let Some(ramfs_filename) = ramfs_filename {
        let ramfs: RamFs = RamFs::open(Path::new(ramfs_filename))?;
        let (ramfs_base, ramfs_size) =
            ramfs.load_into_virtual_memory(vmem, initrd_end, extra_remap_regions)?;
        Ok(LoadedRamFs::Single {
            ramfs,
            base: ramfs_base,
            size: ramfs_size,
        })
    } else {
        Ok(LoadedRamFs::None)
    }
}
