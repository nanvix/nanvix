// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// User Memory Layout
//==================================================================================================

pub mod memory_layout {
    use crate::mm::VirtualAddress;
    pub use config::memory_layout::*;

    ///
    /// # Description
    ///
    /// Base address of the kernel.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    /// - When changing this, linked scripts should also be updated.
    ///
    pub const KERNEL_BASE: VirtualAddress = VirtualAddress::new(KERNEL_BASE_RAW);

    ///
    /// # Description
    ///
    /// End address of the kernel.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    /// - When changing this, linked scripts should also be updated.
    ///
    pub const KERNEL_END: VirtualAddress = VirtualAddress::new(KERNEL_END_RAW);

    ///
    /// # Description
    ///
    /// Base address of the kernel pool.
    ///
    /// # Notes
    ///
    /// - This should be aligned page table boundaries.
    /// - When changing this, required
    ///
    pub const KPOOL_BASE: VirtualAddress = VirtualAddress::new(KPOOL_BASE_RAW);

    ///
    /// # Description
    ///
    /// Base address of user space.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    /// - When changing this, linked scripts should also be updated.
    ///
    pub const USER_BASE: VirtualAddress = VirtualAddress::new(USER_BASE_RAW);

    ///
    /// # Description
    ///
    /// End address of user space.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_END: VirtualAddress = VirtualAddress::new(USER_END_RAW);

    ///
    /// # Description
    ///
    /// Base address of user stack.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_STACK_BASE: VirtualAddress = VirtualAddress::new(USER_STACK_BASE_RAW);

    ///
    /// # Description
    ///
    /// Base address for memory-mapped objects.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_BASE: VirtualAddress = VirtualAddress::new(USER_MMAP_BASE_RAW);

    ///
    /// # Description
    ///
    /// End address for memory-mapped objects.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_MMAP_END: VirtualAddress = VirtualAddress::new(USER_MMAP_END_RAW);
}
