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
    /// Section handle returned by `CreateFileMappingW`. Set to `HANDLE::default()` when the
    /// middle segment is committed (non-page-aligned file fallback) instead of file-backed.
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

        // Replace the placeholder with committed memory. This allocation can later be freed
        // back to a placeholder with VirtualFree(MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER).
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
    /// Replaces a sub-region of guest memory with a zero-copy, file-backed mapping using
    /// Windows placeholder APIs.
    ///
    /// The committed region is freed back to a placeholder, split into up to three segments,
    /// and the file is mapped directly at `self.ptr.add(start)` via `MapViewOfFile3` with
    /// `MEM_REPLACE_PLACEHOLDER`. The committed non-file segments are restored from a saved
    /// copy, and all segments are (re-)registered with the WHP partition.
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

        // Validate page alignment.
        let page_size: usize = ::arch::mem::PAGE_SIZE;
        if !start.is_multiple_of(page_size) {
            let reason: String = format!(
                "start address is not page-aligned (start={start:#x}, page_size={page_size:#x})"
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        // Page-align the size upward: WHP GPA operations require page-aligned sizes, and the
        // file-backed view must cover the full aligned range to avoid exposing unmapped pages.
        let aligned_len: usize = (len + page_size - 1) & !(page_size - 1);
        let tail_start: usize = start + aligned_len;

        // ── 1. Save the non-RAMFS memory content ────────────────────────────────────────────
        //
        // Freeing the committed region destroys all data. Save the head [0..start) and
        // tail [start+aligned_len..size) segments so we can restore them after re-commit.
        let head_backup: Vec<u8> = if start > 0 {
            unsafe { slice::from_raw_parts(self.ptr, start).to_vec() }
        } else {
            Vec::new()
        };
        let tail_backup: Vec<u8> = if tail_start < self.size {
            unsafe {
                slice::from_raw_parts(self.ptr.add(tail_start), self.size - tail_start).to_vec()
            }
        } else {
            Vec::new()
        };

        // ── 2. Unmap the entire GPA range from WHP ─────────────────────────────────────────
        //
        // The region was mapped as a single block in `new()`. All of it must be unmapped before
        // we can free and split the underlying host allocation.
        unsafe {
            WHvUnmapGpaRange(self.partition_handle, 0, self.size as u64).map_err(|e| {
                let reason: String = format!(
                    "failed to unmap GPA range for file remap (size={:#x}, error={e:?})",
                    self.size
                );
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // ── 3. Free the committed region and re-reserve as a placeholder ────────────────────
        //
        // Windows does not support converting a VirtualAlloc2 committed region back to a
        // placeholder via MEM_PRESERVE_PLACEHOLDER. Instead, we release the committed region
        // entirely and immediately re-reserve the same address range as a fresh placeholder.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };
        unsafe {
            VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).map_err(|e| {
                let reason: String = format!("failed to free committed region (error={e:?})");
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        let new_placeholder: *mut u8 = unsafe {
            VirtualAlloc2(
                Some(current_process),
                Some(self.ptr.cast()),
                self.size,
                MEM_RESERVE | MEM_RESERVE_PLACEHOLDER,
                PAGE_NOACCESS.0,
                None,
            )
            .cast::<u8>()
        };

        if new_placeholder.is_null() || new_placeholder != self.ptr {
            let reason: String = format!(
                "failed to re-reserve placeholder at original address (ptr={:?}, \
                 new_placeholder={new_placeholder:?})",
                self.ptr
            );
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        // ── 4. Split the placeholder ────────────────────────────────────────────────────────
        //
        // Split into up to three segments: [0..start), [start..tail_start), [tail_start..size).
        // `VirtualFree(addr, first_half_size, MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER)` splits
        // the placeholder at `addr` into [addr..addr+first_half_size) and the remainder.
        const MEM_RELEASE_PRESERVE: VIRTUAL_FREE_TYPE =
            VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | 0x2 /* MEM_PRESERVE_PLACEHOLDER */);
        if start > 0 {
            unsafe {
                VirtualFree(self.ptr.cast(), start, MEM_RELEASE_PRESERVE).map_err(|e| {
                    let reason: String =
                        format!("failed to split head placeholder (start={start:#x}, error={e:?})");
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }
        if tail_start < self.size {
            unsafe {
                VirtualFree(self.ptr.add(start).cast(), aligned_len, MEM_RELEASE_PRESERVE)
                    .map_err(|e| {
                        let reason: String = format!(
                            "failed to split tail placeholder (start={start:#x}, \
                             aligned_len={aligned_len:#x}, error={e:?})"
                        );
                        error!("remap_file_at(): {reason}");
                        anyhow::anyhow!(reason)
                    })?;
            }
        }

        // ── 5. Re-commit the head segment ───────────────────────────────────────────────────
        if start > 0 {
            let committed: *mut std::ffi::c_void = unsafe {
                VirtualAlloc2(
                    Some(current_process),
                    Some(self.ptr.cast()),
                    start,
                    MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
                    PAGE_READWRITE.0,
                    None,
                )
            };
            if committed.is_null() {
                let reason: String = format!("failed to re-commit head segment [0..{start:#x})");
                error!("remap_file_at(): {reason}");
                anyhow::bail!(reason);
            }
            // Restore saved data.
            unsafe {
                ptr::copy_nonoverlapping(head_backup.as_ptr(), self.ptr, start);
            }
        }

        // ── 6. Map or load the file into the middle placeholder ────────────────────────────
        //
        // When the file size is an exact multiple of the page size, we can create a
        // PAGE_WRITECOPY section whose size equals `aligned_len` and replace the placeholder
        // with a true zero-copy file-backed view (MapViewOfFile3 + MEM_REPLACE_PLACEHOLDER).
        //
        // When the file size is NOT page-aligned, CreateFileMappingW(PAGE_WRITECOPY) refuses
        // to create a section larger than the file (the file is opened read-only and cannot be
        // extended). In that case we fall back to committing the placeholder via VirtualAlloc2
        // and reading the file content directly into the committed region. The rest of the
        // aligned page is implicitly zero-filled by VirtualAlloc2.
        let file_handle: HANDLE = HANDLE(file.as_raw_handle());
        let section_handle: HANDLE = if len.is_multiple_of(page_size) {
            // Zero-copy path: file is page-aligned.
            let size_high: u32 = (aligned_len >> 32) as u32;
            let size_low: u32 = aligned_len as u32;
            let section: HANDLE = unsafe {
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
                let win_err: u32 = unsafe { ::windows::Win32::Foundation::GetLastError().0 };
                unsafe {
                    let _ = CloseHandle(section);
                }
                let reason: String = format!(
                    "MapViewOfFile3 returned null (start={start:#x}, \
                     aligned_len={aligned_len:#x}, win32_error={win_err})"
                );
                error!("remap_file_at(): {reason}");
                anyhow::bail!(reason);
            }

            section
        } else {
            // Fallback path: file is not page-aligned — commit and read.
            debug!(
                "remap_file_at(): file size ({len:#x}) is not page-aligned, using commit+read \
                 fallback"
            );
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
                error!("remap_file_at(): {reason}");
                anyhow::bail!(reason);
            }

            // Read the file content directly into the committed guest memory.
            let dest: &mut [u8] = unsafe { slice::from_raw_parts_mut(self.ptr.add(start), len) };
            (&*file).read_exact(dest).map_err(|e| {
                let reason: String =
                    format!("failed to read file into committed region (error={e:?})");
                error!("remap_file_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;

            // Signal that the middle segment is committed, not file-mapped.
            HANDLE::default()
        };

        // ── 7. Re-commit the tail segment ───────────────────────────────────────────────────
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
                    format!("failed to re-commit tail segment [{tail_start:#x}..{:#x})", self.size);
                error!("remap_file_at(): {reason}");
                anyhow::bail!(reason);
            }
            // Restore saved data.
            unsafe {
                ptr::copy_nonoverlapping(tail_backup.as_ptr(), self.ptr.add(tail_start), tail_size);
            }
        }

        // ── 8. Re-map all segments into the WHP partition ───────────────────────────────────
        //
        // After placeholder splitting, each segment is a separate host allocation. Register
        // them individually with the WHP partition.
        if start > 0 {
            unsafe {
                WHvMapGpaRange(
                    self.partition_handle,
                    self.ptr as *const std::ffi::c_void,
                    0,
                    start as u64,
                    GPA_RWX,
                )
                .map_err(|e| {
                    let reason: String = format!("failed to re-map head GPA range (error={e:?})");
                    error!("remap_file_at(): {reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }

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
        match self.file_remap.take() {
            Some(remap) => {
                // The region was split into up to three segments by `remap_file_at()`.
                // Free each segment individually: committed segments via VirtualFree,
                // the file view via UnmapViewOfFileEx (or VirtualFree if committed), and
                // the section handle via CloseHandle (when file-mapped).
                let tail_start: usize = remap.start + remap.aligned_len;

                // Free the head committed segment.
                if remap.start > 0 {
                    unsafe {
                        if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for head segment");
                        }
                    }
                }

                // Release the middle segment.
                if remap.section_handle == HANDLE::default() {
                    // Middle is committed memory (non-page-aligned file fallback).
                    unsafe {
                        if VirtualFree(self.ptr.add(remap.start).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for committed middle segment");
                        }
                    }
                } else {
                    // Middle is a file-backed view — unmap and close the section handle.
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

                // Free the tail committed segment.
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
