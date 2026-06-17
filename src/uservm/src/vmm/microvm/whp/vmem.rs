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
    warn,
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
        E_HANDLE,
        HANDLE,
    },
    System::{
        Hypervisor::{
            WHV_ADVISE_GPA_RANGE_POPULATE,
            WHV_ADVISE_GPA_RANGE_POPULATE_FLAGS,
            WHV_MAP_GPA_RANGE_FLAGS,
            WHV_MEMORY_RANGE_ENTRY,
            WHV_PARTITION_HANDLE,
            WHvAdviseGpaRange,
            WHvAdviseGpaRangeCodePopulate,
            WHvMapGpaRange,
            WHvMapGpaRangeFlagExecute,
            WHvMapGpaRangeFlagRead,
            WHvMapGpaRangeFlagWrite,
            WHvMemoryAccessRead,
            WHvMemoryAccessWrite,
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
            PrefetchVirtualMemory,
            UNMAP_VIEW_OF_FILE_FLAGS,
            UnmapViewOfFileEx,
            VIRTUAL_FREE_TYPE,
            VirtualAlloc2,
            VirtualFree,
            WIN32_MEMORY_RANGE_ENTRY,
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
    /// Multiple file-backed remap info for multi-image zero-copy mapping, or `None` when
    /// `remap_files_at` has not been called.
    multi_file_remap: Option<MultiFileRemap>,
    /// File handles that must stay alive for file-backed mappings to remain valid.
    backing_files: Vec<File>,
}

///
/// # Description
///
/// Tracks the coordinates of a remap performed by `remap_file_at()`. The middle segment
/// is file-backed via `MapViewOfFile3` with `MEM_REPLACE_PLACEHOLDER`.
/// Used by `Drop` to correctly tear down the split region.
///
struct FileRemap {
    /// Byte offset of the remapped view within the guest memory region.
    start: usize,
    /// Page-aligned size of the remapped view.
    len: usize,
    /// Section handle returned by `CreateFileMappingW`. Set to `HANDLE::default()` (null) when
    /// `remap_file_at()` failed before `map_file_view()` completed, leaving the middle segment
    /// as a placeholder. `Drop` uses this to select between `UnmapViewOfFileEx` (file-backed)
    /// and `VirtualFree` (placeholder) for cleanup.
    section_handle: HANDLE,
}

///
/// # Description
///
/// Tracks the state of a multi-file remap performed by `remap_files_at()`. Multiple sub-regions
/// of guest memory are replaced with file-backed views via placeholder splitting.
///
struct MultiFileRemap {
    /// Byte offset where the placeholder split begins (everything before is committed).
    split_start: usize,
    /// Ordered list of file-backed view segments.
    views: Vec<FileView>,
}

/// A single file-backed view within a multi-file remap.
struct FileView {
    /// Byte offset within the guest memory region.
    offset: usize,
    /// Page-aligned size of the view.
    len: usize,
    /// Section handle for cleanup (default/null if not yet mapped).
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
// from any thread), optional `FileRemap`/`MultiFileRemap` containing OS handles with no thread
// affinity, and a `Vec<File>` of backing file handles. All operations that mutate or deallocate
// the region require exclusive access (`&mut self`), and resources are released exactly once
// during `Drop`. Synchronisation of concurrent access to the pointed-to memory is the
// responsibility of higher-level code.
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
const SPARSE_PAGE_INDEX_SIZE: usize = mem::size_of::<u32>();

/// WHP GPA mapping flags: Read | Write | Execute.
const GPA_RWX: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(
    WHvMapGpaRangeFlagRead.0 | WHvMapGpaRangeFlagWrite.0 | WHvMapGpaRangeFlagExecute.0,
);

/// Combined `MEM_RELEASE | MEM_PRESERVE_PLACEHOLDER` flag used to split a committed region
/// back into placeholders without destroying the head.
const MEM_RELEASE_PRESERVE: VIRTUAL_FREE_TYPE =
    VIRTUAL_FREE_TYPE(MEM_RELEASE.0 | MEM_PRESERVE_PLACEHOLDER.0);

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
            multi_file_remap: None,
            backing_files: Vec::new(),
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
    /// - `file`: File to map from (size must be page-aligned).
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
    pub fn remap_file_at(&mut self, start: usize, file: &File) -> Result<()> {
        if self.file_remap.is_some() || self.multi_file_remap.is_some() {
            let reason: &str = "remap has already been performed on this VirtualMemory";
            error!("remap_file_at(): {reason}");
            anyhow::bail!(reason);
        }

        let view_specs: Vec<(usize, usize)> = self.validate_remap_regions(&[(start, file)])?;
        let len: usize = view_specs[0].1;
        trace!("remap_file_at(): start={start:#x}, len={len:#x}");

        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle that is always valid.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };

        // Phase 1: Unmap the GPA tail from WHP and split the committed region into
        //          placeholders: [start..start+len) and [start+len..size).
        self.prepare_placeholders(start, len)?;

        // Record the split immediately so `Drop` can clean up the placeholders if a later
        // phase fails.  The section handle starts as null (placeholder) and is updated to the
        // real handle after a successful zero-copy map.
        self.file_remap = Some(FileRemap {
            start,
            len,
            section_handle: HANDLE::default(),
        });

        // Phase 2: Replace the middle placeholder [start..start+len) with a zero-copy
        //          file-backed view via MapViewOfFile3.
        let file_handle: HANDLE = HANDLE(file.as_raw_handle());
        let section_handle: HANDLE =
            self.map_file_view(start, len, file_handle, current_process)?;

        // Update the section handle now that the file view is established.
        self.file_remap.as_mut().unwrap().section_handle = section_handle;

        // Phase 3: Re-commit the tail placeholder and re-register all affected GPA segments
        //          with the WHP partition.
        let view: FileView = FileView {
            offset: start,
            len,
            section_handle,
        };
        self.recommit_tail_and_register_gpa(start, &[view], start + len, current_process)?;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps the GPA range `[start..size)` from the WHP partition and converts the committed
    /// host memory in that range back to placeholders, splitting the region into up to two
    /// placeholders: `[start..start+len)` for the file view and
    /// `[start+len..size)` for the tail.
    ///
    fn prepare_placeholders(&mut self, start: usize, len: usize) -> Result<()> {
        let tail_start: usize = start + len;

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
            // `self.ptr.add(start)` is the base of that placeholder, and `len` is
            // within it.  This splits it into [start..tail_start) and [tail_start..size).
            unsafe {
                VirtualFree(self.ptr.add(start).cast(), len, MEM_RELEASE_PRESERVE).map_err(
                    |e| {
                        let reason: String = format!(
                            "failed to split file placeholder (start={start:#x}, len={len:#x}, \
                             error={e:?})"
                        );
                        error!("prepare_placeholders(): {reason}");
                        anyhow::anyhow!(reason)
                    },
                )?;
            }
        }

        Ok(())
    }

    /// Creates a `PAGE_WRITECOPY` section backed by the file and maps it into the placeholder
    /// at `[start..start+len)` via `MapViewOfFile3`.
    ///
    /// The section size is derived from the file via `CreateFileMappingW(size=0)`.  The file
    /// **must** be page-aligned in size so that the section's footprint exactly matches the
    /// placeholder created by `validate_remap_regions()`.  Non-page-aligned files cannot be
    /// mapped via `MEM_REPLACE_PLACEHOLDER` because the section's maximum size (the raw file
    /// size) would not match the page-rounded placeholder, causing `MapViewOfFile3` to fail.
    ///
    /// Returns the section handle on success.
    fn map_file_view(
        &mut self,
        start: usize,
        len: usize,
        file_handle: HANDLE,
        current_process: HANDLE,
    ) -> Result<HANDLE> {
        // SAFETY: `file_handle` comes from a valid open `File`.  Passing size 0 lets the
        // OS derive the section's maximum size from the file's current size.
        let section: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_WRITECOPY, 0, 0, None).map_err(|e| {
                let reason: String = format!("failed to create file mapping section (error={e:?})");
                error!("map_file_view(): {reason}");
                anyhow::anyhow!(reason)
            })?
        };

        // Map the view with ViewSize = len so that it exactly covers the placeholder.
        //
        // SAFETY: `section` is a valid handle from `CreateFileMappingW`.
        // `self.ptr.add(start)` targets a placeholder of exactly `len` bytes
        // created by `prepare_placeholders()`.  `MEM_REPLACE_PLACEHOLDER` atomically
        // replaces it with a file-backed view.
        let view: MEMORY_MAPPED_VIEW_ADDRESS = unsafe {
            MapViewOfFile3(
                section,
                Some(current_process),
                Some(self.ptr.add(start).cast()),
                0,
                len,
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
                    warn!("map_file_view(): CloseHandle() failed while cleaning up section");
                }
            }
            let reason: String = format!(
                "MapViewOfFile3 returned null (start={start:#x}, len={len:#x}, \
                 win32_error={win_err})"
            );
            error!("map_file_view(): {reason}");
            anyhow::bail!(reason);
        }

        // The file-backed view uses PAGE_WRITECOPY: reads are served from the OS page
        // cache (zero-copy), while writes trigger copy-on-write, creating private pages.
        Ok(section)
    }

    ///
    /// # Description
    ///
    /// Replaces multiple sub-regions of guest memory with zero-copy, file-backed mappings using
    /// Windows placeholder APIs.
    ///
    /// The committed region from the first region's offset onward is freed to placeholders,
    /// then each file is mapped via `MapViewOfFile3` with `MEM_REPLACE_PLACEHOLDER`. Gaps
    /// between file regions and the tail are re-committed. All affected GPA segments are
    /// re-registered with the WHP partition.
    ///
    /// # Parameters
    ///
    /// - `regions`: Slice of `(guest_offset, file)` pairs, sorted by offset. Files must be
    ///   page-aligned in size and non-overlapping.
    ///
    /// # Returns
    ///
    /// On success, returns empty. On failure, returns an error.
    ///
    pub fn remap_files_at(&mut self, regions: &[(usize, &File)]) -> Result<()> {
        if regions.is_empty() {
            return Ok(());
        }

        if self.file_remap.is_some() || self.multi_file_remap.is_some() {
            let reason: &str = "remap has already been performed on this VirtualMemory";
            error!("remap_files_at(): {reason}");
            anyhow::bail!(reason);
        }

        let view_specs: Vec<(usize, usize)> = self.validate_remap_regions(regions)?;
        let split_start: usize = view_specs[0].0;

        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle that is always valid.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };

        // Phase 1+2: Unmap GPA [split_start..size) and free to a single placeholder.
        // When `len == self.size - split_start`, the tail split in step 3 is naturally
        // skipped because `split_start + len == self.size`.
        self.prepare_placeholders(split_start, self.size - split_start)?;

        // Phase 3: Split placeholders, commit gaps, and map each file view.
        let (multi_remap, placeholder_base): (MultiFileRemap, usize) =
            self.split_and_map_views(regions, &view_specs, split_start, current_process)?;

        // Phase 4+5: Re-commit tail and re-register all GPA segments with WHP.
        self.recommit_tail_and_register_gpa(
            multi_remap.split_start,
            &multi_remap.views,
            placeholder_base,
            current_process,
        )?;

        self.multi_file_remap = Some(multi_remap);
        Ok(())
    }

    /// Validates that every region in `regions` has a non-zero, page-aligned size, a
    /// page-aligned offset, and fits within the guest memory bounds. Returns the
    /// `(offset, len)` pairs on success.
    fn validate_remap_regions(&self, regions: &[(usize, &File)]) -> Result<Vec<(usize, usize)>> {
        let page_size: usize = ::arch::mem::PAGE_SIZE;
        let mut view_specs: Vec<(usize, usize)> = Vec::with_capacity(regions.len());

        for &(offset, file) in regions {
            let len: usize = {
                let file_len: u64 = file
                    .metadata()
                    .map_err(|e| {
                        let reason: String = format!("failed to query file metadata (error={e:?})");
                        error!("validate_remap_regions(): {reason}");
                        anyhow::anyhow!(reason)
                    })?
                    .len();
                usize::try_from(file_len).map_err(|_| {
                    let reason: String =
                        format!("file size exceeds platform address space (size={file_len})");
                    error!("validate_remap_regions(): {reason}");
                    anyhow::anyhow!(reason)
                })?
            };
            if len == 0 {
                anyhow::bail!("validate_remap_regions(): cannot remap zero-sized file");
            }
            if !offset.is_multiple_of(page_size) {
                anyhow::bail!(
                    "validate_remap_regions(): offset ({offset:#x}) must be page-aligned"
                );
            }
            // The file must be page-aligned because `MapViewOfFile3` with
            // `MEM_REPLACE_PLACEHOLDER` requires `ViewSize` to exactly match the placeholder
            // and `CreateFileMappingW(PAGE_WRITECOPY, size=0)` derives the section maximum
            // from the raw file size.  A non-page-aligned file would produce a section whose
            // size cannot match a page-granular placeholder.
            if !len.is_multiple_of(page_size) {
                anyhow::bail!(
                    "validate_remap_regions(): file size ({len:#x}) must be page-aligned"
                );
            }
            if offset.checked_add(len).is_none_or(|end| end > self.size) {
                anyhow::bail!(
                    "validate_remap_regions(): region [{offset:#x}..{:#x}) exceeds memory bounds",
                    offset.saturating_add(len)
                );
            }
            view_specs.push((offset, len));
        }

        // Verify that regions are sorted by offset and non-overlapping, which is
        // required by split_and_map_views() for correct placeholder splitting.
        for window in view_specs.windows(2) {
            let (prev_offset, prev_len) = window[0];
            let (next_offset, _) = window[1];
            let prev_end: usize = prev_offset + prev_len;
            if next_offset < prev_end {
                anyhow::bail!(
                    "validate_remap_regions(): regions are not sorted or overlap \
                     (prev=[{prev_offset:#x}..{prev_end:#x}), next_offset={next_offset:#x})"
                );
            }
        }

        Ok(view_specs)
    }

    /// Iterates over the file regions left-to-right, splitting gaps from the placeholder and
    /// re-committing them, then splitting each file-sized placeholder and mapping it via
    /// [`Self::map_file_view`]. Returns the populated [`MultiFileRemap`] and the final
    /// placeholder base offset.
    fn split_and_map_views(
        &mut self,
        regions: &[(usize, &File)],
        view_specs: &[(usize, usize)],
        split_start: usize,
        current_process: HANDLE,
    ) -> Result<(MultiFileRemap, usize)> {
        let mut multi_remap: MultiFileRemap = MultiFileRemap {
            split_start,
            views: Vec::with_capacity(regions.len()),
        };

        // After `prepare_placeholders` we have one placeholder [split_start..size).
        let mut placeholder_base: usize = split_start;

        for (i, &(view_offset, file)) in regions.iter().enumerate() {
            let view_len: usize = view_specs[i].1;
            let view_end: usize = view_offset + view_len;

            // Split off and re-commit a gap [placeholder_base..view_offset) if one exists.
            if view_offset > placeholder_base {
                let gap_size: usize = view_offset - placeholder_base;
                self.commit_gap(placeholder_base, gap_size, current_process)?;
                placeholder_base = view_offset;
            }

            // Split the file region from the remaining placeholder (if more follows).
            if view_end < self.size {
                // SAFETY: `self.ptr.add(placeholder_base)` is the base of the current
                // placeholder, and `view_len` splits off the file-sized region.
                unsafe {
                    VirtualFree(
                        self.ptr.add(placeholder_base).cast(),
                        view_len,
                        MEM_RELEASE_PRESERVE,
                    )
                    .map_err(|e| {
                        let reason: String = format!(
                            "split_and_map_views(): failed to split file placeholder (error={e:?})"
                        );
                        error!("{reason}");
                        anyhow::anyhow!(reason)
                    })?;
                }
            }

            // Map the file view at [view_offset..view_end), reusing the single-file helper.
            let file_handle: HANDLE = HANDLE(file.as_raw_handle());
            let section_handle: HANDLE =
                self.map_file_view(view_offset, view_len, file_handle, current_process)?;

            multi_remap.views.push(FileView {
                offset: view_offset,
                len: view_len,
                section_handle,
            });

            placeholder_base = view_end;
        }

        Ok((multi_remap, placeholder_base))
    }

    /// Splits a gap-sized region from the current placeholder and re-commits it as writable
    /// memory.
    fn commit_gap(&self, base: usize, gap_size: usize, current_process: HANDLE) -> Result<()> {
        // SAFETY: `self.ptr.add(base)` is the start of the current placeholder.
        // Splitting off `gap_size` bytes creates two placeholders.
        unsafe {
            VirtualFree(self.ptr.add(base).cast(), gap_size, MEM_RELEASE_PRESERVE).map_err(
                |e| {
                    let reason: String =
                        format!("commit_gap(): failed to split gap placeholder (error={e:?})");
                    error!("{reason}");
                    anyhow::anyhow!(reason)
                },
            )?;
        }

        // Re-commit the gap as writable memory.
        // SAFETY: `self.ptr.add(base)` targets the gap placeholder just split off.
        let committed: *mut std::ffi::c_void = unsafe {
            VirtualAlloc2(
                Some(current_process),
                Some(self.ptr.add(base).cast()),
                gap_size,
                MEM_RESERVE | MEM_COMMIT | MEM_REPLACE_PLACEHOLDER,
                PAGE_READWRITE.0,
                None,
            )
        };
        if committed.is_null() {
            let reason: String = format!(
                "commit_gap(): failed to re-commit gap [{base:#x}..{:#x})",
                base + gap_size
            );
            error!("{reason}");
            anyhow::bail!(reason);
        }

        Ok(())
    }

    /// Re-commits the tail placeholder `[tail_start..size)` (if any) and re-registers
    /// all affected GPA segments (committed gaps, file-backed views, tail) with the WHP
    /// partition. The head `[0..split_start)` is assumed to still be mapped.
    ///
    /// This is the shared finalization step used by both `remap_file_at` (single view) and
    /// `remap_files_at` (multiple views).
    fn recommit_tail_and_register_gpa(
        &self,
        split_start: usize,
        views: &[FileView],
        tail_start: usize,
        current_process: HANDLE,
    ) -> Result<()> {
        // Re-commit the tail placeholder if any.
        if tail_start < self.size {
            let tail_size: usize = self.size - tail_start;
            // SAFETY: `self.ptr.add(tail_start)` targets the tail placeholder.
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
                let reason: String = format!(
                    "recommit_tail_and_register_gpa(): failed to re-commit tail \
                     [{tail_start:#x}..{:#x})",
                    self.size
                );
                error!("{reason}");
                anyhow::bail!(reason);
            }
        }

        // Re-register GPA segments with WHP. The head [0..split_start) was never unmapped.
        let mut prev_end: usize = split_start;
        for view in views {
            // Re-map committed gap before this view.
            if view.offset > prev_end {
                let gap_size: usize = view.offset - prev_end;
                // SAFETY: `self.ptr.add(prev_end)` points to a re-committed gap segment.
                unsafe {
                    WHvMapGpaRange(
                        self.partition_handle,
                        self.ptr.add(prev_end) as *const std::ffi::c_void,
                        prev_end as u64,
                        gap_size as u64,
                        GPA_RWX,
                    )
                    .map_err(|e| {
                        let reason: String = format!(
                            "recommit_tail_and_register_gpa(): failed to map gap GPA (error={e:?})"
                        );
                        error!("{reason}");
                        anyhow::anyhow!(reason)
                    })?;
                }
            }
            // Re-map file-backed view with RWX. The initrd region contains an executable
            // ELF, so execute permission is required. RAMFS-only views do not need execute
            // but granting it uniformly avoids per-region flag plumbing.
            // SAFETY: `self.ptr.add(view.offset)` points to the file-backed view.
            unsafe {
                WHvMapGpaRange(
                    self.partition_handle,
                    self.ptr.add(view.offset) as *const std::ffi::c_void,
                    view.offset as u64,
                    view.len as u64,
                    GPA_RWX,
                )
                .map_err(|e| {
                    let reason: String = format!(
                        "recommit_tail_and_register_gpa(): failed to map file GPA (error={e:?})"
                    );
                    error!("{reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
            prev_end = view.offset + view.len;
        }

        // Re-map committed tail.
        if prev_end < self.size {
            let tail_size: usize = self.size - prev_end;
            // SAFETY: `self.ptr.add(prev_end)` points to the re-committed tail segment.
            unsafe {
                WHvMapGpaRange(
                    self.partition_handle,
                    self.ptr.add(prev_end) as *const std::ffi::c_void,
                    prev_end as u64,
                    tail_size as u64,
                    GPA_RWX,
                )
                .map_err(|e| {
                    let reason: String = format!(
                        "recommit_tail_and_register_gpa(): failed to map tail GPA (error={e:?})"
                    );
                    error!("{reason}");
                    anyhow::anyhow!(reason)
                })?;
            }
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Pre-populates EPT (Extended Page Table) entries for the given GPA ranges using a single
    /// `WHvAdviseGpaRange` call with `WHvAdviseGpaRangeCodePopulate`. This faults in SLAT entries
    /// from the host side before guest execution, avoiding costly EPT violations during
    /// `WHvRunVirtualProcessor`.
    ///
    /// Pre-populating moves this cost to partition setup time where it is measured separately and
    /// does not inflate guest execution latency.
    ///
    /// Issuing a single call for all ranges lets the hypervisor batch the SLAT walk instead of
    /// re-entering the kernel per range.
    ///
    /// # Parameters
    ///
    /// - `gpa_ranges`: Slice of `(gpa, size)` pairs. Each GPA and size must be page-aligned
    ///   and the range must lie within the mapped guest RAM. Zero-sized entries are skipped.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn populate_ept(&self, gpa_ranges: &[(u64, u64)]) -> Result<()> {
        trace!("populate_ept(): {} range(s)", gpa_ranges.len());

        let page_size: u64 = ::arch::mem::PAGE_SIZE as u64;
        let ram_size: u64 = self.size as u64;

        // Validate every range and collect non-empty entries.
        let mut ranges: Vec<WHV_MEMORY_RANGE_ENTRY> = Vec::with_capacity(gpa_ranges.len());
        for &(gpa, size) in gpa_ranges {
            if size == 0 {
                continue;
            }

            if gpa % page_size != 0 || size % page_size != 0 {
                let reason: String =
                    format!("gpa and size must be page-aligned (gpa={gpa:#x}, size={size:#x})");
                error!("populate_ept(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            if gpa.checked_add(size).is_none_or(|end| end > ram_size) {
                let reason: String = format!(
                    "range exceeds mapped guest RAM (gpa={gpa:#x}, size={size:#x}, \
                     ram_size={ram_size:#x})"
                );
                error!("populate_ept(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }

            ranges.push(WHV_MEMORY_RANGE_ENTRY {
                GuestAddress: gpa,
                SizeInBytes: size,
            });
        }

        if ranges.is_empty() {
            return Ok(());
        }

        let populate: WHV_ADVISE_GPA_RANGE_POPULATE = WHV_ADVISE_GPA_RANGE_POPULATE {
            Flags: WHV_ADVISE_GPA_RANGE_POPULATE_FLAGS { AsUINT32: 0 },
            // Pre-fault with write access so the first guest write does not
            // incur an additional fault. This advisory access type does not
            // imply execute permission.
            AccessType: WHvMemoryAccessWrite,
        };

        // SAFETY: `self.partition_handle` is a valid WHP handle from `new()`. Every GPA range
        // in `ranges` lies within the mapped region (bounds checked above). `ranges` and
        // `populate` are stack-local data with the correct layout expected by the API.
        // The buffer pointer and size match `WHV_ADVISE_GPA_RANGE_POPULATE`.
        unsafe {
            WHvAdviseGpaRange(
                self.partition_handle,
                &ranges,
                WHvAdviseGpaRangeCodePopulate,
                (&populate as *const WHV_ADVISE_GPA_RANGE_POPULATE).cast::<std::ffi::c_void>(),
                mem::size_of::<WHV_ADVISE_GPA_RANGE_POPULATE>() as u32,
            )
            .map_err(|e| {
                let reason: String = format!(
                    "WHvAdviseGpaRange(Populate) failed ({} range(s), error={e:?})",
                    ranges.len()
                );
                error!("populate_ept(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(())
    }

    /// Pre-populates EPT entries for the given GPA ranges with **read-only** access.
    ///
    /// This variant uses `WHvMemoryAccessRead` instead of `WHvMemoryAccessWrite`, which
    /// is essential for `PAGE_WRITECOPY` file-backed mappings: read access brings pages into
    /// the working set and the SLAT without triggering copy-on-write.
    pub fn populate_ept_read(&self, ranges_in: &[(u64, u64)]) -> Result<()> {
        let page_size: u64 = ::arch::mem::PAGE_SIZE as u64;
        let ram_size: u64 = self.size as u64;
        let mut ranges: Vec<WHV_MEMORY_RANGE_ENTRY> = Vec::with_capacity(ranges_in.len());

        for &(gpa, size) in ranges_in {
            if size == 0 {
                continue;
            }
            if gpa % page_size != 0 || size % page_size != 0 {
                let reason: String =
                    format!("gpa and size must be page-aligned (gpa={gpa:#x}, size={size:#x})");
                error!("populate_ept_read(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }
            if gpa.checked_add(size).is_none_or(|end| end > ram_size) {
                let reason: String = format!(
                    "range exceeds mapped guest RAM (gpa={gpa:#x}, size={size:#x}, \
                     ram_size={ram_size:#x})"
                );
                error!("populate_ept_read(): {reason}");
                return Err(anyhow::anyhow!(reason));
            }
            ranges.push(WHV_MEMORY_RANGE_ENTRY {
                GuestAddress: gpa,
                SizeInBytes: size,
            });
        }

        if ranges.is_empty() {
            return Ok(());
        }

        let populate: WHV_ADVISE_GPA_RANGE_POPULATE = WHV_ADVISE_GPA_RANGE_POPULATE {
            Flags: WHV_ADVISE_GPA_RANGE_POPULATE_FLAGS { AsUINT32: 0 },
            AccessType: WHvMemoryAccessRead,
        };

        // SAFETY: Same preconditions as `populate_ept`.
        unsafe {
            WHvAdviseGpaRange(
                self.partition_handle,
                &ranges,
                WHvAdviseGpaRangeCodePopulate,
                (&populate as *const WHV_ADVISE_GPA_RANGE_POPULATE).cast::<std::ffi::c_void>(),
                mem::size_of::<WHV_ADVISE_GPA_RANGE_POPULATE>() as u32,
            )
            .map_err(|e| {
                let reason: String = format!(
                    "WHvAdviseGpaRange(Populate/Read) failed ({} range(s), error={e:?})",
                    ranges.len()
                );
                error!("populate_ept_read(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Pre-faults host-side virtual pages for a sub-region of guest memory using
    /// `PrefetchVirtualMemory`. This is the Windows equivalent of `madvise(MADV_WILLNEED)`:
    /// it encourages the OS to bring the backing pages into physical memory ahead of time,
    /// reducing page-fault stalls during guest execution.
    ///
    /// # Parameters
    ///
    /// - `start`: Byte offset from the start of the mapping (must be page-aligned).
    /// - `len`: Size of the region in bytes.
    ///
    /// # Returns
    ///
    /// Upon success, returns empty. Otherwise, returns an error.
    ///
    pub fn prefault_at(&self, start: usize, len: usize) -> Result<()> {
        trace!("prefault_at(): start={start:#x}, len={len:#x}");

        if len == 0 {
            return Ok(());
        }

        let page_size: usize = ::arch::mem::PAGE_SIZE;

        if !start.is_multiple_of(page_size) {
            let reason: String =
                format!("start offset {start:#x} is not page-aligned (page_size={page_size:#x})");
            error!("prefault_at(): {reason}");
            anyhow::bail!(reason);
        }

        if start.checked_add(len).is_none_or(|end| end > self.size) {
            let reason: String = format!(
                "prefault region [{start:#x}, {:#x}) exceeds mapping bounds (size={:#x})",
                start.saturating_add(len),
                self.size
            );
            error!("prefault_at(): {reason}");
            anyhow::bail!(reason);
        }

        // SAFETY: `start` has been bounds-checked so `self.ptr.add(start)` stays within the
        // allocated region. `GetCurrentProcess()` returns a valid pseudo-handle.
        // `WIN32_MEMORY_RANGE_ENTRY` describes the virtual address range to prefetch.
        let entry: WIN32_MEMORY_RANGE_ENTRY = WIN32_MEMORY_RANGE_ENTRY {
            VirtualAddress: unsafe { self.ptr.add(start).cast() },
            NumberOfBytes: len,
        };

        // SAFETY: The entry describes a valid committed or file-backed region within
        // `self.ptr`. The function is advisory and does not modify memory contents.
        unsafe {
            PrefetchVirtualMemory(GetCurrentProcess(), &[entry], 0).map_err(|e| {
                let reason: String = format!(
                    "PrefetchVirtualMemory failed (start={start:#x}, len={len:#x}, error={e:?})"
                );
                error!("prefault_at(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Attaches multiple backing file handles whose memory-mapped regions must remain valid
    /// for the VM's lifetime. Used by the multi-image RAMFS path where each sub-image file
    /// is mapped individually.
    ///
    /// # Parameters
    ///
    /// - `files`: File handles to keep alive.
    ///
    pub fn attach_backing_files(&mut self, files: Vec<File>) {
        self.backing_files.extend(files);
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
    /// **Format (v2 — index-then-data):**
    /// - `u64` — memory size in bytes (little-endian).
    /// - `u32` — number of non-zero pages (little-endian).
    /// - For each non-zero page: `u32` page index (LE).
    /// - Zero-padding to the next `SPARSE_PAGE_SIZE` boundary.
    /// - Contiguous page data: `PAGE_SIZE` bytes × non-zero page count.
    ///
    /// Separating indices from data allows the load path to memory-map the data section and
    /// coalesce contiguous page runs into single large copies.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the snapshot file.
    ///
    /// # Returns
    ///
    /// Upon success, this method returns empty. Otherwise, it returns an error.
    ///
    #[allow(dead_code)]
    pub fn save_snapshot_sparse(&self, path: &Path) -> Result<()> {
        trace!("save_snapshot_sparse(): writing to {:?}", path);

        let page_size: usize = SPARSE_PAGE_SIZE;
        let page_count: usize = self.size / page_size;
        let zero_page: [u8; SPARSE_PAGE_SIZE] = [0u8; SPARSE_PAGE_SIZE];
        let memory_slice: &[u8] = unsafe { slice::from_raw_parts(self.ptr, self.size) };

        // Phase 1: Collect indices of all non-zero pages.
        let mut indices: Vec<u32> = Vec::new();
        for i in 0..page_count {
            let offset: usize = i * page_size;
            let page: &[u8] = &memory_slice[offset..offset + page_size];
            if page != zero_page {
                indices.push(i as u32);
            }
        }
        let non_zero_count: u32 = indices.len() as u32;

        // Phase 2: Compute the page-aligned data offset.
        let header_and_indices_size: usize = SPARSE_MEMORY_SIZE_FIELD
            + SPARSE_PAGE_INDEX_SIZE
            + non_zero_count as usize * SPARSE_PAGE_INDEX_SIZE;
        let data_offset: usize = header_and_indices_size.div_ceil(page_size) * page_size;

        let mut file: File = File::create(path).map_err(|e| {
            let reason: String = format!("failed creating sparse snapshot file (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Write header: memory_size (u64 LE) + page_count (u32 LE).
        file.write_all(&(self.size as u64).to_le_bytes())
            .map_err(|e| {
                let reason: String = format!("failed writing header memory_size (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        file.write_all(&non_zero_count.to_le_bytes()).map_err(|e| {
            let reason: String = format!("failed writing header page_count (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Write all page indices contiguously.
        for &idx in &indices {
            file.write_all(&idx.to_le_bytes()).map_err(|e| {
                let reason: String = format!("failed writing page index (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Pad to the page-aligned data offset.
        let padding: usize = data_offset - header_and_indices_size;
        if padding > 0 {
            let zeros: Vec<u8> = vec![0u8; padding];
            file.write_all(&zeros).map_err(|e| {
                let reason: String = format!("failed writing alignment padding (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Write all page data contiguously (same order as the index array).
        for &idx in &indices {
            let offset: usize = idx as usize * page_size;
            let page: &[u8] = &memory_slice[offset..offset + page_size];
            file.write_all(page).map_err(|e| {
                let reason: String =
                    format!("failed writing page data for page {idx} (error={e:?})");
                error!("save_snapshot_sparse(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        file.sync_all().map_err(|e| {
            let reason: String = format!("failed to sync sparse snapshot file (error={e:?})");
            error!("save_snapshot_sparse(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        trace!(
            "save_snapshot_sparse(): saved {non_zero_count} non-zero pages ({} data bytes) out of \
             {page_count} total pages",
            non_zero_count as usize * page_size,
        );

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Saves the guest memory to a dense snapshot file — a raw image of the full guest physical
    /// address space written at natural offsets. The file always occupies the full guest memory
    /// size on disk.
    ///
    /// The file size equals `self.size` (the guest memory size), so it can be memory-mapped
    /// directly as guest RAM on restore.
    ///
    /// # Parameters
    ///
    /// - `path`: Path where the dense snapshot file will be created.
    ///
    pub fn save_snapshot_dense(&self, path: &Path) -> Result<()> {
        trace!("save_snapshot_dense(): writing {} bytes to {:?}", self.size, path);

        let mut file: File = File::create(path).map_err(|e| {
            let reason: String = format!("failed creating dense snapshot file (error={e:?})");
            error!("save_snapshot_dense(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        // Write the entire guest memory as a single contiguous block.
        // SAFETY: `self.ptr` points to `self.size` bytes of committed memory allocated by
        // `VirtualAlloc2` in `new()`. The region is valid for reads.
        let memory_slice: &[u8] = unsafe { slice::from_raw_parts(self.ptr, self.size) };
        file.write_all(memory_slice).map_err(|e| {
            let reason: String = format!("failed writing dense snapshot (error={e:?})");
            error!("save_snapshot_dense(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        file.sync_all().map_err(|e| {
            let reason: String = format!("failed syncing dense snapshot (error={e:?})");
            error!("save_snapshot_dense(): {reason}");
            anyhow::anyhow!(reason)
        })?;

        trace!("save_snapshot_dense(): wrote {} bytes", self.size);
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Loads a dense snapshot file by mapping it directly as guest physical memory using
    /// copy-on-write (COW) semantics.
    ///
    /// Instead of reading the file and copying pages into committed memory, this method replaces
    /// the committed guest memory allocation with a file-backed view via `MapViewOfFile3` with
    /// `PAGE_WRITECOPY`. Reads are served directly from the OS page cache (zero-copy), while
    /// writes trigger COW, creating private pages on demand.
    ///
    /// This eliminates the bulk data copy entirely, reducing snapshot load from ~65ms to <1ms.
    ///
    /// # Parameters
    ///
    /// - `path`: Path to the dense snapshot file (must be exactly `self.size` bytes).
    ///
    /// # Note
    ///
    /// This method is destructive: it unmaps the GPA range and converts committed memory to a
    /// placeholder before the file mapping is established. If a later phase fails, the
    /// `VirtualMemory` instance may be left in an unusable state. Callers should treat a failure
    /// as fatal for the VM instance.
    ///
    pub fn load_snapshot_cow(&mut self, path: &Path) -> Result<()> {
        if self.file_remap.is_some() || self.multi_file_remap.is_some() {
            let reason: &str = "remap has already been performed on this VirtualMemory";
            error!("load_snapshot_cow(): {reason}");
            anyhow::bail!(reason);
        }

        trace!("load_snapshot_cow(): mapping {:?} as guest memory", path);

        let file: File = File::open(path).map_err(|e| {
            let reason: String = format!("failed opening dense snapshot file (error={e:?})");
            error!("load_snapshot_cow(): {reason}");
            anyhow::anyhow!(reason)
        })?;
        let file_size: u64 = file
            .metadata()
            .map_err(|e| {
                let reason: String = format!("failed reading file metadata (error={e:?})");
                error!("load_snapshot_cow(): {reason}");
                anyhow::anyhow!(reason)
            })?
            .len();

        if file_size as usize != self.size {
            anyhow::bail!(
                "dense snapshot size mismatch: expected {} bytes, got {} bytes",
                self.size,
                file_size
            );
        }

        let file_handle: HANDLE = HANDLE(file.as_raw_handle());
        // SAFETY: `GetCurrentProcess()` returns a pseudo-handle that is always valid.
        let current_process: HANDLE = unsafe { GetCurrentProcess() };

        // Phase 1: Unmap the entire GPA range from the WHP partition.
        // SAFETY: The GPA range [0..size) was mapped during `new()` and is still valid.
        unsafe {
            WHvUnmapGpaRange(self.partition_handle, 0, self.size as u64).map_err(|e| {
                let reason: String = format!("failed to unmap GPA range (error={e:?})");
                error!("load_snapshot_cow(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Phase 2: Convert the committed memory back to a placeholder.
        // SAFETY: `self.ptr` is a committed region originally allocated from a placeholder
        // via `VirtualAlloc2(MEM_REPLACE_PLACEHOLDER)` in `new()`. `MEM_RELEASE |
        // MEM_PRESERVE_PLACEHOLDER` decommits it and converts it back to a placeholder
        // at the same address.
        unsafe {
            VirtualFree(self.ptr.cast(), self.size, MEM_RELEASE_PRESERVE).map_err(|e| {
                let reason: String =
                    format!("failed to convert committed memory to placeholder (error={e:?})");
                error!("load_snapshot_cow(): {reason}");
                anyhow::anyhow!(reason)
            })?;
        }

        // Phase 3: Create a file mapping section with PAGE_WRITECOPY.
        // SAFETY: `file_handle` comes from a valid open `File`. Size 0 lets the OS derive
        // the section size from the file. PAGE_WRITECOPY allows read + copy-on-write access.
        let section: HANDLE = unsafe {
            CreateFileMappingW(file_handle, None, PAGE_WRITECOPY, 0, 0, None).map_err(|e| {
                let reason: String =
                    format!("failed to create file mapping for COW snapshot (error={e:?})");
                error!("load_snapshot_cow(): {reason}");
                anyhow::anyhow!(reason)
            })?
        };

        // Phase 4: Map the file into the placeholder, replacing it with a file-backed view.
        // SAFETY: `section` is a valid handle. `self.ptr` is a placeholder of exactly
        // `self.size` bytes. `MEM_REPLACE_PLACEHOLDER` atomically replaces the placeholder
        // with a file-backed view. `PAGE_WRITECOPY` enables COW semantics.
        let view: MEMORY_MAPPED_VIEW_ADDRESS = unsafe {
            MapViewOfFile3(
                section,
                Some(current_process),
                Some(self.ptr.cast()),
                0,
                self.size,
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
                "MapViewOfFile3 returned null for COW snapshot (size={:#x}, win32_error={win_err})",
                self.size
            );
            error!("load_snapshot_cow(): {reason}");
            anyhow::bail!(reason);
        }

        // Phase 5: Re-register the file-backed memory with the WHP partition.
        // SAFETY: `self.ptr` now points to a valid file-backed mapping of `self.size` bytes.
        // WHP EPT entries handle execute permission independently of host page protection.
        unsafe {
            if let Err(e) = WHvMapGpaRange(
                self.partition_handle,
                self.ptr as *const std::ffi::c_void,
                0,
                self.size as u64,
                GPA_RWX,
            ) {
                // Cleanup: unmap the file view and close the section handle so Drop does not
                // encounter a file-backed region without a corresponding `file_remap` entry.
                let view_addr: MEMORY_MAPPED_VIEW_ADDRESS = MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.ptr.cast(),
                };
                let _ = UnmapViewOfFileEx(view_addr, UNMAP_VIEW_OF_FILE_FLAGS(0));
                let _ = CloseHandle(section);
                let reason: String =
                    format!("failed to re-register GPA after COW mapping (error={e:?})");
                error!("load_snapshot_cow(): {reason}");
                anyhow::bail!(reason);
            }
        }

        // Track the file-backed mapping for correct cleanup in Drop.
        self.file_remap = Some(FileRemap {
            start: 0,
            len: self.size,
            section_handle: section,
        });

        // Keep the file alive — the mapping requires the file handle to remain valid.
        self.backing_files.push(file);

        trace!("load_snapshot_cow(): mapped {} bytes as COW guest memory", self.size);

        Ok(())
    }
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        // Unmap the entire GPA range from the WHP partition before releasing host memory. If the
        // partition was already destroyed (drop ordering between `Arc<VirtualMemory>` and
        // `Arc<WhpPartition>`), `WHvUnmapGpaRange` returns `E_HANDLE`. That outcome is benign
        // because `WHvDeletePartition` implicitly tears down all GPA mappings, so we silently
        // ignore it. Any other failure is unexpected and is logged as an error so it remains
        // visible without aborting teardown.
        // SAFETY: `self.partition_handle` was a valid WHP handle when `new()` returned. By
        // the time `drop` runs the partition may already have been destroyed, in which case
        // the handle is stale; `WHvUnmapGpaRange` detects that and returns `E_HANDLE`
        // without dereferencing host memory, so the call is sound either way. The GPA range
        // [0..size) was mapped during construction and has not been freed.
        unsafe {
            if let Err(e) = WHvUnmapGpaRange(self.partition_handle, 0, self.size as u64)
                && e.code() != E_HANDLE
            {
                error!("WHvUnmapGpaRange() failed in Drop (error={e:?})");
            }
        }

        if let Some(multi) = self.multi_file_remap.take() {
            // Multi-file remap active. The region was split into: committed head, N file views
            // (possibly with committed gaps between them), and a committed tail.

            // Free the committed head [0..split_start).
            if multi.split_start > 0 {
                unsafe {
                    if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed for head segment (multi)");
                    }
                }
            }

            // Free committed gaps and file views, left to right.
            let mut prev_end: usize = multi.split_start;
            for view in &multi.views {
                // Free committed gap [prev_end..view.offset) if any.
                if view.offset > prev_end {
                    unsafe {
                        if VirtualFree(self.ptr.add(prev_end).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for gap segment (multi)");
                        }
                    }
                }
                // Release file-backed view.
                if view.section_handle == HANDLE::default() {
                    unsafe {
                        if VirtualFree(self.ptr.add(view.offset).cast(), 0, MEM_RELEASE).is_err() {
                            error!("VirtualFree() failed for placeholder segment (multi)");
                        }
                    }
                } else {
                    unsafe {
                        let view_addr: MEMORY_MAPPED_VIEW_ADDRESS = MEMORY_MAPPED_VIEW_ADDRESS {
                            Value: self.ptr.add(view.offset).cast(),
                        };
                        if UnmapViewOfFileEx(view_addr, UNMAP_VIEW_OF_FILE_FLAGS(0)).is_err() {
                            error!("UnmapViewOfFileEx() failed for file view (multi)");
                        }
                        if CloseHandle(view.section_handle).is_err() {
                            error!("CloseHandle() failed for section handle (multi)");
                        }
                    }
                }
                prev_end = view.offset + view.len;
            }

            // Free committed tail [prev_end..size) if any.
            if prev_end < self.size {
                unsafe {
                    if VirtualFree(self.ptr.add(prev_end).cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed for tail segment (multi)");
                    }
                }
            }
        } else if let Some(remap) = self.file_remap.take() {
            // Single file remap active. The region was split into up to three segments
            // by `remap_file_at()`.
            let tail_start: usize = remap.start + remap.len;

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
                unsafe {
                    if VirtualFree(self.ptr.add(remap.start).cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed for middle placeholder segment");
                    }
                }
            } else {
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

            // Free the tail committed segment (or placeholder).
            if tail_start < self.size {
                unsafe {
                    if VirtualFree(self.ptr.add(tail_start).cast(), 0, MEM_RELEASE).is_err() {
                        error!("VirtualFree() failed for tail segment");
                    }
                }
            }
        } else {
            // No remap: the region is a single committed block.
            unsafe {
                if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                    error!("VirtualFree() failed");
                }
            }
        }
    }
}
