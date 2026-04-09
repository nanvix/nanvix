// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::whp::partition::WhpPartition;
use ::anyhow::Result;
use ::log::{
    error,
    trace,
};
use ::std::{
    fs::File,
    io::{
        Read,
        Write,
    },
    mem,
    os::windows::io::AsRawHandle,
    path::Path,
    ptr,
    slice,
};
use ::windows::Win32::{
    Foundation::{
        CloseHandle,
        HANDLE,
    },
    System::{
        Hypervisor::{
            WHV_MAP_GPA_RANGE_FLAGS,
            WHV_PARTITION_HANDLE,
            WHvMapGpaRange,
            WHvUnmapGpaRange,
        },
        Memory::{
            CreateFileMappingW,
            MEM_COMMIT,
            MEM_RELEASE,
            MEM_REPLACE_PLACEHOLDER,
            MEM_RESERVE,
            MEM_RESERVE_PLACEHOLDER,
            MEMORY_MAPPED_VIEW_ADDRESS,
            MapViewOfFile3,
            PAGE_NOACCESS,
            PAGE_READWRITE,
            PAGE_WRITECOPY,
            UNMAP_VIEW_OF_FILE_FLAGS,
            UnmapViewOfFileEx,
            VIRTUAL_FREE_TYPE,
            VirtualAlloc2,
            VirtualFree,
        },
        Threading::GetCurrentProcess,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A structure that represents the memory of a virtual machine backed by WHP.
///
/// The region is allocated as a placeholder via `VirtualAlloc2(MEM_RESERVE_PLACEHOLDER)` and
/// then committed with `MEM_REPLACE_PLACEHOLDER`. This allows sub-regions to be later split off
/// and replaced with file-backed views (e.g., for zero-copy RAMFS loading) without disturbing the
/// rest of the allocation.
///
pub struct VirtualMemory {
    /// Virtual memory pointer.
    ptr: *mut u8,
    /// Size of the virtual memory.
    size: usize,
    /// WHP partition handle for GPA (un)mapping operations.
    partition_handle: WHV_PARTITION_HANDLE,
    /// File-backed remap info for cleanup, or `None` when the region is a single committed block.
    file_remap: Option<FileRemap>,
}

///
/// # Description
///
/// Tracks the coordinates of a file-backed remap performed by `remap_file_at()`. The sub-region
/// `[start .. start + aligned_len)` has been split from the placeholder and replaced with a
/// `MapViewOfFile3` view. Used by `Drop` to correctly tear down the split region.
///
struct FileRemap {
    /// Byte offset of the remapped view within the guest memory region.
    start: usize,
    /// Page-aligned size of the remapped view.
    aligned_len: usize,
    /// Section handle returned by `CreateFileMappingW`.
    section_handle: HANDLE,
}

///
/// # Description
///
/// A structure that represents the header in virtual memory snapshot files.
///
#[repr(C)]
struct SnapshotHeader {
    /// Memory size (8 bytes): usize.
    memory_size: usize,
}

// SAFETY: `VirtualMemory` owns a contiguous region of virtual memory allocated with
// `VirtualAlloc2` and released in `Drop` (via `VirtualFree` and, when a file remap is active,
// `UnmapViewOfFileEx` + `CloseHandle`), a WHP partition handle (an opaque OS handle safe to use
// from any thread), and an optional `FileRemap` containing OS handles with no thread affinity.
// All operations that mutate or deallocate the region require exclusive access (`&mut self`),
// and resources are released exactly once during `Drop`. Synchronisation of concurrent access to
// the pointed-to memory is the responsibility of higher-level code.
unsafe impl Send for VirtualMemory {}
unsafe impl Sync for VirtualMemory {}

//==================================================================================================
// Constants
//==================================================================================================

const SIZE_OF_HEADER: usize = mem::size_of::<SnapshotHeader>();

/// Page size used by the sparse snapshot format.
const SPARSE_PAGE_SIZE: usize = ::arch::mem::PAGE_SIZE;

/// Byte size of the memory-size field (`u64`) in the sparse snapshot header.
const SPARSE_MEMORY_SIZE_FIELD: usize = mem::size_of::<u64>();

/// Byte size of a page-index entry (`u32`) in the sparse snapshot format.
const SPARSE_PAGE_INDEX_SIZE: usize = ::arch::mem::paging::PageTableEntry::SIZE;

/// WHP GPA mapping flags: Read | Write | Execute.
const GPA_RWX: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(7);

//==================================================================================================
// Implementations
//==================================================================================================

impl VirtualMemory {
    ///
    /// # Description
    ///
    /// Creates a new virtual memory region and maps it into the WHP partition.
    ///
    /// # Parameters
    ///
    /// - `partition`: WHP partition that hosts the virtual machine.
    /// - `size`: Size of the virtual memory.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the function returns the new virtual memory. Otherwise, it
    /// returns an error.
    ///
    pub fn new(partition: &WhpPartition, size: usize) -> Result<Self> {
        trace!("VirtualMemory::new(): size={size}");

        // Reserve the entire address space as a placeholder so that sub-regions can later be
        // split and replaced with file-backed views via MapViewOfFile3.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };
        let placeholder: *mut u8 = unsafe {
            VirtualAlloc2(
                Some(current_process),
                None,
                size,
                MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
                PAGE_NOACCESS.0,
                None,
            )
            .cast::<u8>()
        };

        if placeholder.is_null() {
            let reason: String =
                "failed to reserve placeholder for the virtual machine".to_string();
            error!("VirtualMemory::new(): {reason} (memory_size={size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        // Replace the placeholder with committed read-write memory. The region can still be
        // split back into placeholders later via VirtualFree(MEM_RELEASE | MEM_PRESERVE).
        let ptr: *mut u8 = unsafe {
            VirtualAlloc2(
                Some(current_process),
                Some(placeholder.cast()),
                size,
                MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE.0,
                None,
            )
            .cast::<u8>()
        };

        if ptr.is_null() {
            unsafe {
                let _ = VirtualFree(placeholder.cast(), 0, MEM_RELEASE);
            }
            let reason: String = "failed to commit memory for the virtual machine".to_string();
            error!("VirtualMemory::new(): {reason} (memory_size={size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        // Create the VirtualMemory instance (destructor will free memory on error).
        let vmem: Self = Self {
            ptr,
            size,
            partition_handle: partition.handle(),
            file_remap: None,
        };

        // Map the memory into the WHP partition at guest physical address 0.
        unsafe {
            WHvMapGpaRange(
                partition.handle(),
                ptr as *const std::ffi::c_void,
                0, // Guest physical address.
                size as u64,
                GPA_RWX,
            )
            .map_err(|e| {
                let reason: String =
                    format!("failed to map memory into WHP partition (error={e:?})");
                error!("VirtualMemory::new(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(vmem)
    }

    pub fn get_raw_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Writes bytes into the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address in the virtual memory.
    /// - `data`: Data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn write_bytes(&mut self, addr: u64, data: &[u8]) -> Result<()> {
        let addr: usize = match usize::try_from(addr) {
            Ok(v) => v,
            Err(_) => {
                let reason: String = format!("invalid address (addr={addr:#010x})");
                error!("write_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if region lies within the virtual memory (overflow-safe).
        match addr.checked_add(data.len()) {
            Some(end) if end <= self.size => {},
            _ => {
                let reason: String = format!(
                    "invalid memory access (addr={addr:#010x}, len={:#x}, size={:#x})",
                    data.len(),
                    self.size
                );
                error!("write_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(addr), data.len());
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Replaces a sub-region of guest memory with a zero-copy, file-backed mapping using Windows
    /// placeholder APIs. Only the target sub-region is affected; preceding memory (kernel, initrd)
    /// remains committed and untouched.
    ///
    /// The committed region at `[start .. start + aligned_len)` is freed back to a placeholder
    /// (via `MEM_PRESERVE_PLACEHOLDER`), then replaced with a `MapViewOfFile3` file-backed view.
    /// The preceding and following GPA segments are re-registered with WHP individually.
    ///
    /// # Parameters
    ///
    /// - `start`: Byte offset from the start of guest memory (must be page-aligned).
    /// - `len`: Size of the region to remap (in bytes).
    /// - `file`: File to map from.
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn remap_file_at(&mut self, start: usize, len: usize, file: &File) -> Result<()> {
        trace!("remap_file_at(): start={start:#x}, len={len:#x}");

        if len == 0 {
            let reason: &str = "cannot remap zero-sized region";
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        if start.checked_add(len).is_none_or(|end| end > self.size) {
            let reason: String = format!(
                "remap region [{start:#x}, {:#x}) exceeds memory bounds (size={:#x})",
                start.saturating_add(len),
                self.size
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        let page_size: usize = ::arch::mem::PAGE_SIZE;
        if !start.is_multiple_of(page_size) {
            let reason: String = format!(
                "start address is not page-aligned (start={start:#x}, page_size={page_size:#x})"
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        // Page-align the size upward for WHP GPA operations.
        let aligned_len: usize = (len + page_size - 1) & !(page_size - 1);
        if start + aligned_len > self.size {
            let reason: String = format!(
                "page-aligned remap [{start:#x}, {:#x}) exceeds memory bounds (size={:#x})",
                start + aligned_len,
                self.size
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        let tail_start: usize = start + aligned_len;
        let current_process: HANDLE = unsafe { GetCurrentProcess() };

        // ── 1. Unmap the GPA sub-regions that will be affected ─────────────────────────────
        //
        // Unmap [start..size) from the WHP partition. The head [0..start) stays mapped and its
        // committed memory is untouched.
        let unmap_size: u64 = (self.size - start) as u64;
        unsafe {
            WHvUnmapGpaRange(self.partition_handle, start as u64, unmap_size).map_err(|e| {
                let reason: String = format!(
                    "failed to unmap GPA range for file remap (start={start:#x}, \
                     size={unmap_size:#x}, error={e:?})"
                );
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // ── 2. Free [start..size) back to a placeholder ────────────────────────────────────
        //
        // `VirtualFree(addr, size, MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER)` splits the
        // committed region: [0..start) stays committed, [start..size) becomes a placeholder.
        const MEM_RELEASE_PRESERVE: VIRTUAL_FREE_TYPE =
            VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | 0x2 /* MEM_PRESERVE_PLACEHOLDER */);
        unsafe {
            VirtualFree(self.ptr.add(start).cast(), self.size - start, MEM_RELEASE_PRESERVE)
                .map_err(|e| {
                    let reason: String = format!(
                        "failed to free tail to placeholder (start={start:#x}, error={e:?})"
                    );
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
        }

        // ── 3. Split the tail placeholder if there is memory after the file region ─────────
        //
        // After step 2, [start..size) is a single placeholder. If `tail_start < size`, split
        // it into [start..tail_start) and [tail_start..size).
        if tail_start < self.size {
            unsafe {
                VirtualFree(self.ptr.add(start).cast(), aligned_len, MEM_RELEASE_PRESERVE)
                    .map_err(|e| {
                        let reason: String = format!(
                            "failed to split file placeholder (start={start:#x}, \
                             aligned_len={aligned_len:#x}, error={e:?})"
                        );
                        error!("remap_file_at(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;
            }
        }

        // ── 4. Map the file into the middle placeholder ────────────────────────────────────
        //
        // Create a PAGE_WRITECOPY section and atomically replace the placeholder with a
        // file-backed view via MapViewOfFile3(MEM_REPLACE_PLACEHOLDER).
        let file_handle: HANDLE = HANDLE(file.as_raw_handle());
        let size_high: u32 = (aligned_len >> 32) as u32;
        let size_low: u32 = aligned_len as u32;
        let section_handle: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_WRITECOPY, size_high, size_low, None)
                .map_err(|e| {
                    let reason: String =
                        format!("failed to create file mapping section (error={e:?})");
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?
        };

        let view: MEMORY_MAPPED_VIEW_ADDRESS = unsafe {
            MapViewOfFile3(
                section_handle,
                Some(current_process),
                Some(self.ptr.add(start).cast()),
                0,
                aligned_len,
                MEM_REPLACE_PLACEHOLDER,
                PAGE_WRITECOPY.0,
                None,
            )
        };

        if view.Value.is_null() {
            unsafe {
                let _ = CloseHandle(section_handle);
            }
            let reason: String = format!(
                "MapViewOfFile3 returned null (start={start:#x}, aligned_len={aligned_len:#x})"
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        // ── 5. Re-commit the tail placeholder (if any) ────────────────────────────────────
        if tail_start < self.size {
            let tail_size: usize = self.size - tail_start;
            let committed: *mut std::ffi::c_void = unsafe {
                VirtualAlloc2(
                    Some(current_process),
                    Some(self.ptr.add(tail_start).cast()),
                    tail_size,
                    MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
                    PAGE_READWRITE.0,
                    None,
                )
            };
            if committed.is_null() {
                let reason: String =
                    format!("failed to re-commit tail [{tail_start:#x}..{:#x})", self.size);
                error!("remap_file_at(): {reason}");
                anyhow::bail!(reason);
            }
        }

        // ── 6. Re-map affected segments into the WHP partition ─────────────────────────────
        //
        // The head [0..start) was never unmapped from WHP or freed — skip it.
        // Map the file-backed middle segment.
        unsafe {
            WHvMapGpaRange(
                self.partition_handle,
                self.ptr.add(start) as *const std::ffi::c_void,
                start as u64,
                aligned_len as u64,
                GPA_RWX,
            )
            .map_err(|e| {
                let reason: String = format!(
                    "failed to map file-backed GPA range (start={start:#x}, len={aligned_len:#x}, \
                     error={e:?})"
                );
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Map the tail committed segment (if any).
        if tail_start < self.size {
            let tail_size: usize = self.size - tail_start;
            unsafe {
                WHvMapGpaRange(
                    self.partition_handle,
                    self.ptr.add(tail_start) as *const std::ffi::c_void,
                    tail_start as u64,
                    tail_size as u64,
                    GPA_RWX,
                )
                .map_err(|e| {
                    let reason: String = format!("failed to re-map tail GPA range (error={e:?})");
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }

        // Record the remap coordinates for Drop.
        self.file_remap = Some(FileRemap {
            start,
            aligned_len,
            section_handle,
        });

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Reads bytes from the virtual memory.
    ///
    /// # Parameters
    ///
    /// - `addr`: Address in the virtual memory.
    /// - `data`: Buffer to read into.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn read_bytes(&self, addr: u64, data: &mut [u8]) -> Result<()> {
        let addr: usize = match usize::try_from(addr) {
            Ok(v) => v,
            Err(_) => {
                let reason: String = format!("invalid address (addr={addr:#010x})");
                error!("read_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if region lies within the virtual memory (overflow-safe).
        match addr.checked_add(data.len()) {
            Some(end) if end <= self.size => {},
            _ => {
                let reason: String = format!(
                    "invalid memory access (addr={addr:#010x}, len={:#x}, size={:#x})",
                    data.len(),
                    self.size
                );
                error!("read_bytes(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        }

        unsafe {
            ptr::copy_nonoverlapping(self.ptr.add(addr), data.as_mut_ptr(), data.len());
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Saves the current state of the virtual memory to a snapshot file.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn save_snapshot(&self, path: &Path) -> Result<()> {
        trace!("save_snapshot(): writing to {:?}", path);

        let mut file: File = match File::create(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed creating virtual memory snapshot file (error={e:?})");
                error!("save_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let header: SnapshotHeader = SnapshotHeader {
            memory_size: self.size,
        };

        // SAFETY: `SnapshotHeader` is `#[repr(C)]` plain-old-data with no padding or
        // invalid bit patterns. We create a byte slice of exactly `SIZE_OF_HEADER` bytes
        // from a live, properly aligned reference, which is safe to write out.
        let header_bytes: &[u8] = unsafe {
            slice::from_raw_parts((&header as *const SnapshotHeader).cast::<u8>(), SIZE_OF_HEADER)
        };

        if let Err(e) = file.write_all(header_bytes) {
            let reason: String =
                format!("failed writing the header to virtual memory snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        let memory_slice: &[u8] = unsafe { slice::from_raw_parts(self.ptr, self.size) };
        if let Err(e) = file.write_all(memory_slice) {
            let reason: String = format!("failed to write memory contents (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        if let Err(e) = file.sync_all() {
            let reason: String = format!("failed to sync snapshot file (error={e:?})");
            error!("save_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads a virtual memory snapshot from a snapshot file.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn load_snapshot(&mut self, path: &Path) -> Result<()> {
        trace!("load_snapshot(): reading from {:?}", path);

        let mut file: File = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                let reason: String =
                    format!("failed opening virtual memory snapshot file (error={e:?})");
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        let mut header_bytes: [u8; SIZE_OF_HEADER] = [0u8; SIZE_OF_HEADER];
        match file.read_exact(&mut header_bytes) {
            Ok(()) => {},
            Err(e) => {
                let reason: String = format!(
                    "failed reading header from virtual memory snapshot file (error={e:?})"
                );
                error!("load_snapshot(): {reason}");
                anyhow::bail!(reason)
            },
        };

        // SAFETY: `SnapshotHeader` is `#[repr(C)]` with a single `usize` field, so every
        // bit pattern of `SIZE_OF_HEADER` bytes is a valid `SnapshotHeader` (no invalid
        // representations). We have read exactly `SIZE_OF_HEADER` bytes from the file.
        let header: SnapshotHeader =
            unsafe { ptr::read_unaligned(header_bytes.as_ptr().cast::<SnapshotHeader>()) };

        if header.memory_size != self.size {
            let reason: String =
                format!("memory size mismatch: expected {}, got {}", self.size, header.memory_size);
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        let memory_slice: &mut [u8] = unsafe { slice::from_raw_parts_mut(self.ptr, self.size) };
        if let Err(e) = file.read_exact(memory_slice) {
            let reason: String = format!("failed to read memory contents (error={e:?})");
            error!("load_snapshot(): {reason}");
            anyhow::bail!(reason)
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Saves a sparse virtual memory snapshot, writing only pages with non-zero content.
    ///
    /// **Format:**
    /// - `u64` — memory size in bytes (little-endian).
    /// - `u32` — number of non-zero pages (little-endian).
    /// - For each non-zero page: `u32` page index (LE) + PAGE_SIZE raw bytes.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn save_snapshot_sparse(&self, path: &Path) -> Result<()> {
        trace!("save_snapshot_sparse(): writing to {:?}", path);

        let page_size: usize = SPARSE_PAGE_SIZE;
        let page_count: usize = self.size / page_size;
        let zero_page: [u8; SPARSE_PAGE_SIZE] = [0u8; SPARSE_PAGE_SIZE];
        let memory_slice: &[u8] = unsafe { slice::from_raw_parts(self.ptr, self.size) };

        let mut file: File = File::create(path).map_err(|e| {
            let reason: String = format!("failed creating sparse snapshot file (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Write placeholder header (memory_size + page_count).
        file.write_all(&(self.size as u64).to_le_bytes())
            .map_err(|e| {
                let reason: String = format!("failed writing header memory_size (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        file.write_all(&0u32.to_le_bytes()).map_err(|e| {
            let reason: String =
                format!("failed writing header placeholder page_count (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        let mut non_zero_count: u32 = 0;
        for i in 0..page_count {
            let offset: usize = i * page_size;
            let page: &[u8] = &memory_slice[offset..offset + page_size];
            if page != zero_page {
                file.write_all(&(i as u32).to_le_bytes()).map_err(|e| {
                    let reason: String = format!("failed writing page index {i} (error={e:?})");
                    error!("save_snapshot_sparse(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
                file.write_all(page).map_err(|e| {
                    let reason: String =
                        format!("failed writing page data for page {i} (error={e:?})");
                    error!("save_snapshot_sparse(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
                non_zero_count += 1;
            }
        }

        // Seek back and write actual page count.
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(SPARSE_MEMORY_SIZE_FIELD as u64))
            .map_err(|e| {
                let reason: String =
                    format!("failed seeking to header page_count field (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        file.write_all(&non_zero_count.to_le_bytes()).map_err(|e| {
            let reason: String = format!("failed writing final page_count (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        file.sync_all().map_err(|e| {
            let reason: String = format!("failed to sync sparse snapshot file (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        trace!(
            "save_snapshot_sparse(): saved {non_zero_count} non-zero pages ({} bytes) out of \
             {page_count} total pages",
            non_zero_count as usize * (SPARSE_PAGE_INDEX_SIZE + page_size),
        );

        Ok(())
    }

    /// Loads a sparse virtual memory snapshot.
    ///
    /// Assumes the target memory is zero-initialized (true for fresh `VirtualAlloc`).
    /// Only non-zero pages stored in the snapshot are written.
    pub fn load_snapshot_sparse(&mut self, path: &Path) -> Result<()> {
        trace!("load_snapshot_sparse(): reading from {:?}", path);

        let page_size: usize = SPARSE_PAGE_SIZE;
        let mut file: File = File::open(path).map_err(|e| {
            let reason: String = format!("failed opening sparse snapshot file (error={e:?})");
            error!("load_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Read header.
        let mut size_buf: [u8; 8] = [0u8; 8];
        file.read_exact(&mut size_buf).map_err(|e| {
            let reason: String = format!("failed reading header memory_size (error={e:?})");
            error!("load_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        let memory_size: usize = u64::from_le_bytes(size_buf) as usize;
        if memory_size != self.size {
            anyhow::bail!("memory size mismatch: expected {}, got {}", self.size, memory_size);
        }

        let mut count_buf: [u8; 4] = [0u8; 4];
        file.read_exact(&mut count_buf).map_err(|e| {
            let reason: String = format!("failed reading header page_count (error={e:?})");
            error!("load_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        let page_count: u32 = u32::from_le_bytes(count_buf);

        // Read and restore each non-zero page.
        let mut idx_buf: [u8; 4] = [0u8; 4];
        let mut page_buf: [u8; SPARSE_PAGE_SIZE] = [0u8; SPARSE_PAGE_SIZE];
        for _ in 0..page_count {
            file.read_exact(&mut idx_buf).map_err(|e| {
                let reason: String = format!("failed reading page index (error={e:?})");
                error!("load_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
            let idx: usize = u32::from_le_bytes(idx_buf) as usize;
            file.read_exact(&mut page_buf).map_err(|e| {
                let reason: String =
                    format!("failed reading page data for page {idx} (error={e:?})");
                error!("load_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;

            let offset: usize = idx * page_size;
            if offset + page_size > self.size {
                anyhow::bail!("sparse page index {idx} out of bounds (memory_size={})", self.size);
            }

            unsafe {
                ptr::copy_nonoverlapping(page_buf.as_ptr(), self.ptr.add(offset), page_size);
            }
        }

        trace!(
            "load_snapshot_sparse(): loaded {page_count} pages ({} bytes)",
            page_count as usize * page_size,
        );

        Ok(())
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        match self.file_remap.take() {
            Some(remap) => {
                // The region was split into segments by `remap_file_at()`:
                //   [0..start): committed (untouched by remap)
                //   [start..start+aligned_len): file-backed view
                //   [start+aligned_len..size): committed (re-committed by remap)
                let tail_start: usize = remap.start + remap.aligned_len;

                // Free the head committed segment [0..start).
                if remap.start > 0 {
                    unsafe {
                        if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for head segment");
                        }
                    }
                }

                // Release the file-backed view and close the section handle.
                unsafe {
                    let view: MEMORY_MAPPED_VIEW_ADDRESS = MEMORY_MAPPED_VIEW_ADDRESS {
                        Value: self.ptr.add(remap.start).cast(),
                    };
                    if UnmapViewOfFileEx(view, UNMAP_VIEW_OF_FILE_FLAGS(0)).is_err() {
                        error!("UnmapViewOfFileEx() failed for file view");
                    }
                    if CloseHandle(remap.section_handle).is_err() {
                        error!("CloseHandle() failed for section handle");
                    }
                }

                // Free the tail committed segment [tail_start..size).
                if tail_start < self.size {
                    unsafe {
                        if VirtualFree(self.ptr.add(tail_start).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for tail segment");
                        }
                    }
                }
            },
            None => {
                // No remap: the region is a single committed block.
                unsafe {
                    if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed");
                    }
                }
            },
        }
    }
}
