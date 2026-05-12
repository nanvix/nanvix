// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::hal::mem::PhysicalAddress;

//==================================================================================================
// Structures
//==================================================================================================

pub struct KernelModule {
    /// Start address of the ELF binary data.
    start: PhysicalAddress,
    /// Size of the ELF binary data.
    size: usize,
    /// Base address of the memory region that must be mapped for this module.
    /// For multibinary images this covers the full image (including the header
    /// where cmdline strings reside); for single-binary modules it equals `start`.
    region_base: PhysicalAddress,
    /// Total size of the memory region from `region_base` that must be mapped.
    region_size: usize,
    /// Pointer to the command-line bytes in bootloader-provided RAM.
    /// Stored as raw parts instead of `&'static str` so that `cmdline_bytes_mut()` can hand out
    /// `&mut [u8]` without aliasing a live `&str`.
    cmdline_ptr: *const u8,
    /// Length of the command-line byte region.
    cmdline_len: usize,
}

// SAFETY: the raw pointer originates from a `&'static str` whose referent lives for the entire
// program and is only accessed from a single core with interrupts disabled.
unsafe impl Send for KernelModule {}
unsafe impl Sync for KernelModule {}

impl KernelModule {
    /// Creates a new kernel module whose mapped region matches the ELF extent.
    pub fn new(start: PhysicalAddress, size: usize, cmdline: &'static str) -> Self {
        Self {
            start,
            size,
            region_base: start,
            region_size: size,
            cmdline_ptr: cmdline.as_ptr(),
            cmdline_len: cmdline.len(),
        }
    }

    /// Creates a new kernel module with an explicit memory region base and size.
    pub fn new_with_region(
        start: PhysicalAddress,
        size: usize,
        region_base: PhysicalAddress,
        region_size: usize,
        cmdline: &'static str,
    ) -> Self {
        Self {
            start,
            size,
            region_base,
            region_size,
            cmdline_ptr: cmdline.as_ptr(),
            cmdline_len: cmdline.len(),
        }
    }

    /// Gets the start address of the ELF binary.
    pub fn start(&self) -> PhysicalAddress {
        self.start
    }

    /// Gets the size of the ELF binary.
    pub fn size(&self) -> usize {
        self.size
    }

    /// Gets the base address of the memory region to map.
    pub fn region_base(&self) -> PhysicalAddress {
        self.region_base
    }

    /// Gets the size of the memory region to map.
    pub fn region_size(&self) -> usize {
        self.region_size
    }

    /// Gets the command line of the module.
    ///
    /// # Safety (internal)
    ///
    /// Uses `from_utf8_unchecked` because `cmdline_ptr`/`cmdline_len` originate from a
    /// `&'static str` supplied at construction time, guaranteeing valid UTF-8.
    pub fn cmdline(&self) -> &str {
        // SAFETY: the raw parts were extracted from a `&'static str` in the constructor.
        unsafe {
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                self.cmdline_ptr,
                self.cmdline_len,
            ))
        }
    }

    /// Returns a mutable byte slice over the command-line region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that no shared reference to the same command-line bytes is live.
    /// The underlying memory must be mapped read-write.
    pub unsafe fn cmdline_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.cmdline_ptr as *mut u8, self.cmdline_len) }
    }

    /// Shrinks the effective command-line length after in-place compaction.
    ///
    /// # Panics
    ///
    /// Panics if `new_len` exceeds the current length.
    pub fn set_cmdline_len(&mut self, new_len: usize) {
        assert!(new_len <= self.cmdline_len, "cannot grow cmdline length");
        self.cmdline_len = new_len;
    }
}

impl core::fmt::Debug for KernelModule {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "kernel_module {{ start: {:?}, size: {:?}, cmdline: {:?} }}",
            self.start,
            self.size,
            self.cmdline()
        )
    }
}
