// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::microvm::whp::partition::WhpPartition;
use ::anyhow::Result;
use ::log::{
    error,
    info,
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
    path::Path,
    ptr,
    slice,
};
use windows::Win32::System::{
    Hypervisor::{
        WHV_MAP_GPA_RANGE_FLAGS,
        WHvMapGpaRange,
    },
    Memory::{
        GetLargePageMinimum,
        MEM_COMMIT,
        MEM_LARGE_PAGES,
        MEM_RELEASE,
        MEM_RESERVE,
        PAGE_READWRITE,
        VirtualAlloc,
        VirtualFree,
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
    /// Whether this allocation uses large pages.
    #[allow(dead_code)]
    large_pages: bool,
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

// SAFETY: `VirtualMemory` is a thin owner of a contiguous region of virtual memory allocated
// with `VirtualAlloc` and released in `Drop` via `VirtualFree`. The fields (`ptr: *mut u8` and
// `size: usize`) are plain data without thread-affine state. All operations that mutate or
// deallocate the region require exclusive access (`&mut self`), and the allocation is released
// exactly once during `Drop`. Synchronisation of concurrent access to the pointed-to memory is
// the responsibility of higher-level code.
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

        // Try to allocate with large pages (2MB) for better EPT performance.
        // Large pages reduce EPT entries from size/4KB to size/2MB, dramatically
        // reducing EPT violation overhead on WHP.
        let (ptr, large_pages) = Self::try_alloc_large_pages(size)
            .unwrap_or_else(|| {
                // Fall back to regular 4KB page allocation.
                let p = unsafe {
                    VirtualAlloc(
                        Some(ptr::null()),
                        size,
                        MEM_COMMIT | MEM_RESERVE,
                        PAGE_READWRITE,
                    )
                    .cast::<u8>()
                };
                (p, false)
            });

        if ptr.is_null() {
            let reason: String = "failed to allocate memory for the virtual machine".to_string();
            error!("VirtualMemory::new(): {reason} (memory_size={size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        if large_pages {
            info!(
                "VirtualMemory::new(): allocated {size} bytes with large pages (2MB)",
            );
        } else {
            info!(
                "VirtualMemory::new(): allocated {size} bytes with standard pages (4KB)",
            );
        }

        // Create the VirtualMemory instance (destructor will free memory on error).
        let vmem: Self = Self { ptr, size, large_pages };

        // Map the memory into the WHP partition at guest physical address 0.
        unsafe {
            WHvMapGpaRange(
                partition.handle(),
                ptr as *const std::ffi::c_void,
                0, // Guest physical address.
                size as u64,
                WHV_MAP_GPA_RANGE_FLAGS(7), // Read | Write | Execute.
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

    /// Attempt to allocate memory using large pages (2MB).
    ///
    /// Returns `Some((ptr, true))` on success, `None` if large pages are unavailable.
    /// Requires `SeLockMemoryPrivilege` to be granted to the process user.
    fn try_alloc_large_pages(size: usize) -> Option<(*mut u8, bool)> {
        // Check large page support and minimum size.
        let large_page_min: usize = unsafe { GetLargePageMinimum() };
        if large_page_min == 0 {
            warn!("VirtualMemory: large pages not supported on this system");
            return None;
        }

        // Size must be a multiple of the large page minimum.
        if size % large_page_min != 0 {
            warn!(
                "VirtualMemory: size {size} is not a multiple of large page minimum {large_page_min}"
            );
            return None;
        }

        // Try to allocate with MEM_LARGE_PAGES.
        let ptr: *mut u8 = unsafe {
            VirtualAlloc(
                Some(ptr::null()),
                size,
                MEM_COMMIT | MEM_RESERVE | MEM_LARGE_PAGES,
                PAGE_READWRITE,
            )
            .cast::<u8>()
        };

        if ptr.is_null() {
            warn!(
                "VirtualMemory: large page allocation failed (need SeLockMemoryPrivilege). \
                 Falling back to standard pages"
            );
            None
        } else {
            Some((ptr, true))
        }
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
    /// Reads `len` bytes from a file directly into the virtual memory at the given guest address,
    /// bypassing any intermediate heap buffer.
    ///
    /// # Parameters
    ///
    /// - `addr`: Destination address in the virtual memory.
    /// - `file`: File to read from (starting at its current seek position).
    /// - `len`: Number of bytes to read.
    ///
    /// # Returns
    ///
    /// Upon successful completion, this method returns empty. Otherwise, it returns an error.
    ///
    pub fn load_from_file(&mut self, addr: u64, file: &File, len: usize) -> Result<()> {
        let addr: usize = match usize::try_from(addr) {
            Ok(v) => v,
            Err(_) => {
                let reason: String = format!("invalid address (addr={addr:#010x})");
                error!("load_from_file(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        };

        // Check if region lies within the virtual memory (overflow-safe).
        match addr.checked_add(len) {
            Some(end) if end <= self.size => {},
            _ => {
                let reason: String = format!(
                    "invalid memory access (addr={addr:#010x}, len={len:#x}, size={:#x})",
                    self.size
                );
                error!("load_from_file(): {reason}");
                return Err(anyhow::anyhow!(reason));
            },
        }

        // SAFETY: `self.ptr` is a valid, committed allocation of `self.size` bytes (from
        // `VirtualAlloc`). The checked range validation above guarantees that
        // `[addr, addr + len)` lies within this region.
        let dest: &mut [u8] = unsafe { slice::from_raw_parts_mut(self.ptr.add(addr), len) };

        // Read from `&File` (shared reference) — Rust's std provides `impl Read for &File`,
        // which calls `ReadFile` on the underlying OS handle.
        let mut reader: &File = file;
        reader.read_exact(dest).map_err(|e| {
            let reason: String = format!(
                "failed to read file into virtual memory (addr={addr:#010x}, len={len}, \
                 error={e:?})"
            );
            error!("load_from_file(): {reason}");
            anyhow::anyhow!(reason)
        })?;

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
        unsafe {
            if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                error!("VirtualFree() failed");
            }
        }
    }
}
