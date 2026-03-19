// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::vmm::whp::partition::WhpPartition;
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
        MEM_COMMIT,
        MEM_RELEASE,
        MEM_RESERVE,
        PAGE_READWRITE,
        VirtualAlloc,
        VirtualFree,
    },
};

/// Page size in bytes (4 KiB, matching the x86 architecture).
#[allow(dead_code)]
const PAGE_SIZE: usize = 4096;

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
    /// Pointers to lazily-mapped MMIO dummy pages (freed on drop).
    mmio_pages: Vec<*mut u8>,
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

        // Allocate memory using VirtualAlloc.
        let ptr: *mut u8 = unsafe {
            VirtualAlloc(Some(ptr::null()), size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
                .cast::<u8>()
        };

        if ptr.is_null() {
            let reason: String = "failed to allocate memory for the virtual machine".to_string();
            error!("VirtualMemory::new(): {reason} (memory_size={size:?})");
            return Err(anyhow::anyhow!(reason));
        }

        // Create the VirtualMemory instance (destructor will free memory on error).
        let vmem: Self = Self {
            ptr,
            size,
            mmio_pages: Vec::new(),
        };

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

    pub fn get_raw_ptr(&self) -> *mut u8 {
        self.ptr
    }

    pub fn get_size(&self) -> usize {
        self.size
    }

    ///
    /// # Description
    ///
    /// Lazily maps a zeroed page at the given guest physical address. This allows guest accesses
    /// to unmapped MMIO regions to succeed (reads return zero, writes are discarded).
    ///
    /// # Parameters
    ///
    /// - `partition`: WHP partition that hosts the virtual machine.
    /// - `gpa`: Guest physical address that caused the memory-access exit.
    ///
    pub fn map_mmio_page(&mut self, partition: &WhpPartition, gpa: u64) -> Result<()> {
        let page_gpa: u64 = gpa & !(PAGE_SIZE as u64 - 1);

        // Allocate a zeroed host page.
        let page_ptr: *mut u8 = unsafe {
            VirtualAlloc(Some(ptr::null()), PAGE_SIZE, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
                .cast::<u8>()
        };
        if page_ptr.is_null() {
            anyhow::bail!("map_mmio_page(): failed to allocate MMIO page for GPA {page_gpa:#010x}");
        }

        // If this is the LAPIC register page (0xFEE00000), pre-populate it
        // with valid register values so the kernel's LAPIC init code
        // succeeds without LAPIC emulation.
        const LAPIC_BASE: u64 = 0xFEE0_0000;
        if page_gpa == LAPIC_BASE {
            trace!("map_mmio_page(): populating LAPIC page at GPA {page_gpa:#010x}");
            let page_slice = unsafe { std::slice::from_raw_parts_mut(page_ptr, PAGE_SIZE) };
            let write_u32 = |s: &mut [u8], off: usize, val: u32| {
                s[off..off + 4].copy_from_slice(&val.to_le_bytes());
            };
            // APIC ID: 0 (processor 0).
            write_u32(page_slice, 0x20, 0);
            // APIC Version: 0x50014 (version 0x14, max LVT entries 5).
            write_u32(page_slice, 0x30, 0x0005_0014);
            // SVR: 0xFF (APIC disabled, spurious vector 0xFF — kernel default).
            write_u32(page_slice, 0xF0, 0xFF);
        }

        // Map the page into the WHP partition.
        let result = unsafe {
            WHvMapGpaRange(
                partition.handle(),
                page_ptr as *const std::ffi::c_void,
                page_gpa,
                PAGE_SIZE as u64,
                WHV_MAP_GPA_RANGE_FLAGS(7), // Read | Write | Execute.
            )
        };

        if let Err(e) = result {
            // Free the page on mapping failure.
            unsafe {
                let _ = VirtualFree(page_ptr.cast(), 0, MEM_RELEASE);
            }
            anyhow::bail!(
                "map_mmio_page(): failed to map MMIO page at GPA {page_gpa:#010x} (error={e:?})"
            );
        }

        trace!("map_mmio_page(): mapped zeroed page at GPA {page_gpa:#010x}");
        self.mmio_pages.push(page_ptr);
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

        // Check if region lies within the virtual memory.
        if addr + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("write_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
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

        // Check if region lies within the virtual memory.
        if addr + data.len() > self.size {
            let reason: String = format!("invalid memory access (addr={addr:#010x})");
            error!("read_bytes(): {reason}");
            return Err(anyhow::anyhow!(reason));
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
}

impl Drop for VirtualMemory {
    fn drop(&mut self) {
        // Free MMIO dummy pages first.
        for page_ptr in self.mmio_pages.drain(..) {
            unsafe {
                if VirtualFree(page_ptr.cast(), 0, MEM_RELEASE).is_err() {
                    error!("VirtualFree() failed for MMIO page");
                }
            }
        }
        unsafe {
            if VirtualFree(self.ptr.cast(), 0, MEM_RELEASE).is_err() {
                error!("VirtualFree() failed");
            }
        }
    }
}
