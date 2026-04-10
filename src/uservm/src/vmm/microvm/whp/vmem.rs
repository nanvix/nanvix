// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::whp::partition::WhpPartition;
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    trace,
    warn,
};
use ::std::{
    fs::File,
    io::{
        Read,
        Seek,
        SeekFrom,
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
            WHvMapGpaRangeFlagExecute,
            WHvMapGpaRangeFlagRead,
            WHvMapGpaRangeFlagWrite,
            WHvUnmapGpaRange,
        },
        Memory::{
            CreateFileMappingW,
            MEM_COMMIT,
            MEM_PRESERVE_PLACEHOLDER,
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
pub struct VirtualMemory {
    /// Virtual memory pointer.
    ptr: *mut u8,
    /// Size of the virtual memory.
    size: usize,
    /// WHP partition handle for GPA (un)mapping operations.
    partition_handle: WHV_PARTITION_HANDLE,
    /// File-backed remap info for placeholder-split cleanup, or `None` when the region is a
    /// single committed block.
    file_remap: Option<FileRemap>,
}

///
/// # Description
///
/// Tracks the coordinates of a remap performed by `remap_file_at()`. The middle segment
/// is either file-backed (via `MapViewOfFile3` with `MEM_REPLACE_PLACEHOLDER`) when the file
/// is page-aligned, or committed (via `VirtualAlloc2` with `MEM_REPLACE_PLACEHOLDER`) when
/// the file is not page-aligned. Used by `Drop` to correctly tear down the split region.
///
struct FileRemap {
    /// Byte offset of the remapped view within the guest memory region.
    start: usize,
    /// Page-aligned size of the remapped view.
    aligned_len: usize,
    /// Section handle returned by `CreateFileMappingW`. Set to `HANDLE::default()` (null) when
    /// the middle segment uses the commit+read fallback (non-page-aligned file) instead of a
    /// file-backed view. `Drop` uses this to select between `UnmapViewOfFileEx` (file-backed)
    /// and `VirtualFree` (committed) for cleanup.
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
const GPA_RWX: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(
    WHvMapGpaRangeFlagRead.0 | WHvMapGpaRangeFlagWrite.0 | WHvMapGpaRangeFlagExecute.0,
);

/// WHP GPA mapping flags: Read | Write (no Execute).
const GPA_RW: WHV_MAP_GPA_RANGE_FLAGS =
    WHV_MAP_GPA_RANGE_FLAGS(WHvMapGpaRangeFlagRead.0 | WHvMapGpaRangeFlagWrite.0);

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
        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle that is always valid.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };
        // SAFETY: Allocates a new placeholder region from the OS with no pre-existing pointer
        // dependency.  All parameters are valid constants; `None` base address lets the OS choose.
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

        // Replace the placeholder with committed memory. This allocation can later be freed
        // back to a placeholder with VirtualFree(MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER).
        // SAFETY: `placeholder` is a valid placeholder region returned by the `VirtualAlloc2`
        // call above.  `MEM_REPLACE_PLACEHOLDER` atomically replaces it with committed memory.
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
            // Release the placeholder before returning.
            // SAFETY: `placeholder` is a valid reserved region from the earlier `VirtualAlloc2`;
            // `MEM_RELEASE` with size 0 releases the entire allocation.
            unsafe {
                if VirtualFree(placeholder.cast(), 0, MEM_RELEASE).is_err() {
                    warn!("VirtualMemory::new(): VirtualFree() failed while releasing placeholder");
                }
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
        // SAFETY: `ptr` is a valid committed region of `size` bytes from `VirtualAlloc2`.
        // `partition.handle()` is a valid WHP partition handle.
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
    /// Replaces a sub-region of guest memory with a zero-copy, file-backed mapping using
    /// Windows placeholder APIs.
    ///
    /// Only the tail of the committed region (from `start` onward) is freed back to a
    /// placeholder via `MEM_PRESERVE_PLACEHOLDER`, which splits the committed region in place
    /// without disturbing preceding memory (kernel, initrd). The file is then mapped directly
    /// at `self.ptr.add(start)` via `MapViewOfFile3` with `MEM_REPLACE_PLACEHOLDER`, and the
    /// affected GPA segments are re-registered with the WHP partition.
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
    /// # Note
    ///
    /// If this method fails partway through, the guest memory region may be left in an
    /// inconsistent state (partially unmapped from WHP, partially split). This mirrors the
    /// Linux `mmap(MAP_FIXED)` semantics where the previous mapping is destroyed before the
    /// new one is established. Callers should treat a failure as fatal for the VM instance.
    ///
    pub fn remap_file_at(&mut self, start: usize, len: usize, file: &File) -> Result<()> {
        trace!("remap_file_at(): start={start:#x}, len={len:#x}");

        if self.file_remap.is_some() {
            let reason: &str = "remap_file_at() has already been called on this VirtualMemory";
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

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

        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle that is always valid.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };

        // Phase 1: Unmap the GPA tail from WHP and split the committed region into
        //          placeholders: [start..start+aligned_len) and [start+aligned_len..size).
        self.prepare_placeholders(start, aligned_len)?;

        // Record the split immediately so `Drop` can clean up the placeholders if a later
        // phase fails.  The section handle starts as null (placeholder / committed fallback)
        // and is updated to the real handle after a successful zero-copy map.
        self.file_remap = Some(FileRemap {
            start,
            aligned_len,
            section_handle: HANDLE::default(),
        });

        // Phase 2: Replace the middle placeholder [start..start+aligned_len) with a
        //          file-backed view (zero-copy) or committed memory (fallback).
        let section_handle: HANDLE =
            self.replace_placeholder_with_file(start, len, aligned_len, file, current_process)?;

        // Update the section handle now that the file view is established.
        self.file_remap.as_mut().unwrap().section_handle = section_handle;

        // Phase 3: Re-commit the tail placeholder and re-register all affected GPA segments
        //          with the WHP partition.
        self.commit_and_map_tail(start, aligned_len, current_process)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps the GPA range `[start..size)` from the WHP partition and converts the committed
    /// host memory in that range back to placeholders, splitting the region into up to two
    /// placeholders: `[start..start+aligned_len)` for the file view and
    /// `[start+aligned_len..size)` for the tail.
    ///
    fn prepare_placeholders(&mut self, start: usize, aligned_len: usize) -> Result<()> {
        let tail_start: usize = start + aligned_len;

        // ── 1. Unmap [start..size) from the WHP partition ──────────────────────────────────
        //
        // The head [0..start) stays mapped and its committed memory is untouched, avoiding
        // the cost of backing up and restoring kernel/initrd data.
        let unmap_size: u64 = (self.size - start) as u64;
        // SAFETY: `self.partition_handle` is a valid WHP handle from `new()`. The GPA range
        // [start..size) was mapped in `new()` and has not yet been unmapped.
        unsafe {
            WHvUnmapGpaRange(self.partition_handle, start as u64, unmap_size).map_err(|e| {
                let reason: String = format!(
                    "failed to unmap GPA range for file remap (start={start:#x}, \
                     size={unmap_size:#x}, error={e:?})"
                );
                error!("prepare_placeholders(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // ── 2. Free [start..size) back to a placeholder ────────────────────────────────────
        //
        // `VirtualFree(addr, size, MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER)` splits the
        // committed region in place: [0..start) stays committed, [start..size) becomes a
        // placeholder. No data is destroyed in the head.
        const MEM_RELEASE_PRESERVE: VIRTUAL_FREE_TYPE =
            VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | MEM_PRESERVE_PLACEHOLDER.0);
        // SAFETY: `self.ptr.add(start)` points within the committed region (bounds checked
        // by the caller). `MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER` splits the committed
        // allocation: [0..start) stays committed, [start..size) becomes a placeholder.
        unsafe {
            VirtualFree(self.ptr.add(start).cast(), self.size - start, MEM_RELEASE_PRESERVE)
                .map_err(|e| {
                    let reason: String = format!(
                        "failed to free tail to placeholder (start={start:#x}, error={e:?})"
                    );
                    error!("prepare_placeholders(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
        }

        // ── 3. Split the tail placeholder if there is memory after the file region ─────────
        //
        // After step 2, [start..size) is a single placeholder. If `tail_start < size`, split
        // it into [start..tail_start) and [tail_start..size).
        if tail_start < self.size {
            // SAFETY: After step 2, [start..size) is a single placeholder.
            // `self.ptr.add(start)` is the base of that placeholder, and `aligned_len` is
            // within it.  This splits it into [start..tail_start) and [tail_start..size).
            unsafe {
                VirtualFree(self.ptr.add(start).cast(), aligned_len, MEM_RELEASE_PRESERVE)
                    .map_err(|e| {
                        let reason: String = format!(
                            "failed to split file placeholder (start={start:#x}, \
                             aligned_len={aligned_len:#x}, error={e:?})"
                        );
                        error!("prepare_placeholders(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Replaces the placeholder at `[start..start+aligned_len)` with a file-backed view.
    ///
    /// When the file size (`len`) is page-aligned, a true zero-copy `PAGE_WRITECOPY` section
    /// is created and mapped via `MapViewOfFile3`. When the file size is NOT page-aligned,
    /// the placeholder is committed via `VirtualAlloc2` and the file content is read into it.
    ///
    /// Returns the section handle for the file-backed path, or `HANDLE::default()` for the
    /// commit+read fallback (used by `Drop` to choose the correct cleanup).
    ///
    fn replace_placeholder_with_file(
        &mut self,
        start: usize,
        len: usize,
        aligned_len: usize,
        file: &File,
        current_process: HANDLE,
    ) -> Result<HANDLE> {
        let page_size: usize = ::arch::mem::PAGE_SIZE;
        let file_handle: HANDLE = HANDLE(file.as_raw_handle());

        if len.is_multiple_of(page_size) {
            self.map_file_view_zero_copy(start, aligned_len, file_handle, current_process)
        } else {
            self.map_file_view_commit_read(start, len, aligned_len, file, current_process)
        }
    }

    /// Zero-copy path: creates a `PAGE_WRITECOPY` section backed by the file and maps it into
    /// the placeholder at `[start..start+aligned_len)` via `MapViewOfFile3`.
    ///
    /// Returns the section handle on success.
    fn map_file_view_zero_copy(
        &mut self,
        start: usize,
        aligned_len: usize,
        file_handle: HANDLE,
        current_process: HANDLE,
    ) -> Result<HANDLE> {
        let size_high: u32 = (aligned_len >> 32) as u32;
        let size_low: u32 = aligned_len as u32;

        // SAFETY: `file_handle` comes from a valid open `File`. Size parameters are
        // derived from `aligned_len` which does not exceed the region bounds.
        let section: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_WRITECOPY, size_high, size_low, None)
                .map_err(|e| {
                    let reason: String =
                        format!("failed to create file mapping section (error={e:?})");
                    error!("map_file_view_zero_copy(): {reason}");
                    anyhow::anyhow!(reason)
                })?
        };

        // SAFETY: `section` is a valid handle from `CreateFileMappingW`.
        // `self.ptr.add(start)` targets a placeholder of exactly `aligned_len` bytes
        // created by `prepare_placeholders()`.  `MEM_REPLACE_PLACEHOLDER` atomically
        // replaces it with a file-backed view.
        let view: MEMORY_MAPPED_VIEW_ADDRESS = unsafe {
            MapViewOfFile3(
                section,
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
            // SAFETY: `GetLastError` has no preconditions.
            let win_err: u32 = unsafe { ::windows::Win32::Foundation::GetLastError().0 };
            // SAFETY: `section` is a valid handle that must be closed on this error path.
            unsafe {
                if CloseHandle(section).is_err() {
                    warn!(
                        "map_file_view_zero_copy(): CloseHandle() failed while cleaning up section"
                    );
                }
            }
            let reason: String = format!(
                "MapViewOfFile3 returned null (start={start:#x}, aligned_len={aligned_len:#x}, \
                 win32_error={win_err})"
            );
            error!("map_file_view_zero_copy(): {reason}");
            anyhow::bail!(reason);
        }

        // The file-backed view uses PAGE_WRITECOPY: reads are served from the OS page
        // cache (zero-copy), while writes trigger copy-on-write, creating private pages.
        Ok(section)
    }

    /// Fallback path for non-page-aligned files: commits the placeholder at
    /// `[start..start+aligned_len)` and reads the file content directly into it.
    ///
    /// Returns `HANDLE::default()` to signal this is a committed region, not file-mapped.
    fn map_file_view_commit_read(
        &mut self,
        start: usize,
        len: usize,
        aligned_len: usize,
        file: &File,
        current_process: HANDLE,
    ) -> Result<HANDLE> {
        debug!(
            "map_file_view_commit_read(): file size ({len:#x}) is not page-aligned, using \
             commit+read fallback"
        );

        // SAFETY: `self.ptr.add(start)` targets a placeholder of `aligned_len` bytes.
        // `MEM_REPLACE_PLACEHOLDER` replaces it with committed memory.
        let committed: *mut std::ffi::c_void = unsafe {
            VirtualAlloc2(
                Some(current_process),
                Some(self.ptr.add(start).cast()),
                aligned_len,
                MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE.0,
                None,
            )
        };
        if committed.is_null() {
            let reason: String = format!(
                "failed to commit file placeholder [{start:#x}..{:#x})",
                start + aligned_len
            );
            error!("map_file_view_commit_read(): {reason}");
            anyhow::bail!(reason);
        }

        // Read the file content directly into the committed guest memory.
        // Seek to the start of the file in case a previous operation advanced the cursor.
        (&*file).seek(SeekFrom::Start(0)).map_err(|e| {
            let reason: String = format!("failed to seek to start of file (error={e:?})");
            error!("map_file_view_commit_read(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // SAFETY: The `VirtualAlloc2` above succeeded, so `self.ptr.add(start)` points to
        // `aligned_len` bytes of committed memory. `len <= aligned_len` (checked by caller).
        let dest: &mut [u8] = unsafe { slice::from_raw_parts_mut(self.ptr.add(start), len) };
        (&*file).read_exact(dest).map_err(|e| {
            let reason: String = format!("failed to read file into committed region (error={e:?})");
            error!("map_file_view_commit_read(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        Ok(HANDLE::default())
    }

    ///
    /// # Description
    ///
    /// Re-commits the tail placeholder `[start+aligned_len..size)` (if any) and re-registers
    /// the file-backed middle segment and the tail committed segment with the WHP partition.
    ///
    fn commit_and_map_tail(
        &mut self,
        start: usize,
        aligned_len: usize,
        current_process: HANDLE,
    ) -> Result<()> {
        let tail_start: usize = start + aligned_len;

        // Re-commit the tail placeholder (if any).
        if tail_start < self.size {
            let tail_size: usize = self.size - tail_start;
            // SAFETY: `self.ptr.add(tail_start)` targets the tail placeholder created by
            // `prepare_placeholders()`.  `tail_size` spans [tail_start..size).
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
                    format!("failed to re-commit tail segment [{tail_start:#x}..{:#x})", self.size);
                error!("commit_and_map_tail(): {reason}");
                anyhow::bail!(reason);
            }
        }

        // Re-map the file-backed (or committed) middle segment into the WHP partition.
        // The head [0..start) was never unmapped — skip it.
        // Use RW-only permissions: RAMFS data does not contain executable code.
        // SAFETY: `self.ptr.add(start)` points to the file-backed (or committed) view of
        // `aligned_len` bytes established by `replace_placeholder_with_file()`.
        unsafe {
            WHvMapGpaRange(
                self.partition_handle,
                self.ptr.add(start) as *const std::ffi::c_void,
                start as u64,
                aligned_len as u64,
                GPA_RW,
            )
            .map_err(|e| {
                let reason: String = format!(
                    "failed to map file-backed GPA range (start={start:#x}, len={aligned_len:#x}, \
                     error={e:?})"
                );
                error!("commit_and_map_tail(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Re-map the tail committed segment (if any).
        if tail_start < self.size {
            let tail_size: usize = self.size - tail_start;
            // SAFETY: `self.ptr.add(tail_start)` points to the re-committed tail segment.
            // `tail_size` spans [tail_start..size). The WHP handle is valid.
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
                    error!("commit_and_map_tail(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }

        Ok(())
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
        // Unmap the entire GPA range from the WHP partition before releasing host memory.
        // SAFETY: `self.partition_handle` is a valid WHP handle from `new()`. The GPA range
        // [0..size) was mapped during construction and has not been freed.
        unsafe {
            if let Err(e) = WHvUnmapGpaRange(self.partition_handle, 0, self.size as u64) {
                error!("WHvUnmapGpaRange() failed in Drop (error={e:?})");
            }
        }

        match self.file_remap.take() {
            Some(remap) => {
                // The region was split into up to three segments by `remap_file_at()`.
                // Free each segment individually: committed segments via VirtualFree,
                // the file view via UnmapViewOfFileEx (or VirtualFree if committed), and
                // the section handle via CloseHandle (when file-mapped).
                //
                // This also handles partial-failure states: if `remap_file_at()` failed
                // after `prepare_placeholders()` but before completing all phases, some
                // segments may still be placeholders. `VirtualFree(MEM_RELEASE)` releases
                // both committed regions and placeholders, so cleanup is safe either way.
                let tail_start: usize = remap.start + remap.aligned_len;

                // Free the head committed segment.
                if remap.start > 0 {
                    // SAFETY: `self.ptr` is the base of the original allocation.  After
                    // `remap_file_at()`, [0..start) is a standalone committed region.
                    unsafe {
                        if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for head segment");
                        }
                    }
                }

                // Release the middle segment.
                if remap.section_handle == HANDLE::default() {
                    // Middle is committed memory, a placeholder (partial failure), or
                    // the commit+read fallback (non-page-aligned file). VirtualFree
                    // handles all three cases.
                    // SAFETY: `self.ptr.add(remap.start)` is the base of a standalone
                    // region created during `remap_file_at()`.
                    unsafe {
                        if VirtualFree(self.ptr.add(remap.start).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for committed middle segment");
                        }
                    }
                } else {
                    // Middle is a file-backed view — unmap and close the section handle.
                    // SAFETY: `self.ptr.add(remap.start)` is the address of the file-backed
                    // view created by `MapViewOfFile3` in `remap_file_at()` and
                    // `remap.section_handle` is the corresponding section handle.  Both are
                    // released exactly once here.
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
                }

                // Free the tail committed segment (or placeholder if Phase 3a failed).
                if tail_start < self.size {
                    // SAFETY: `self.ptr.add(tail_start)` is the base of either the
                    // re-committed tail segment or a placeholder (if `commit_and_map_tail()`
                    // failed before re-committing). `VirtualFree(MEM_RELEASE)` handles both.
                    unsafe {
                        if VirtualFree(self.ptr.add(tail_start).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for tail segment");
                        }
                    }
                }
            },
            None => {
                // No remap: the region is a single committed block.
                // SAFETY: `self.ptr` is a valid committed allocation from `VirtualAlloc2`
                // in `new()` and is released exactly once here.
                unsafe {
                    if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed");
                    }
                }
            },
        }
    }
}
