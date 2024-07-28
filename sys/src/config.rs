// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Kernel
//==================================================================================================

pub mod kernel {
    use crate::constants;

    ///
    /// # Description
    ///
    /// Total size of physical memory (in bytes).
    ///
    pub const MEMORY_SIZE: usize = 256 * constants::MEGABYTE;

    ///
    /// # Description
    ///
    /// Total size of the kernel pool (in bytes).
    ///
    /// # Notes
    ///
    /// - This size be a multiple of a page size.
    /// - This size cannot exceed the size of a page table.
    ///
    pub const KPOOL_SIZE: usize = 4 * constants::MEGABYTE;

    ///
    /// # Description
    ///
    /// Kernel stack size (in bytes).
    ///
    /// # Notes
    ///
    /// - This size should be a multiple of a page size.
    /// - This size cannot exceed the size of a page table.
    /// - When changing this boot code should also be updated.
    ///
    pub const KSTACK_SIZE: usize = 8 * 4 * constants::KILOBYTE;

    ///
    /// # Description
    ///
    /// Timer frequency (in Hz).
    ///
    pub const TIMER_FREQ: u32 = 100;

    ///
    /// # Description
    ///
    /// Scheduler frequency (in ticks).
    ///
    /// # Notes
    ///
    /// - This should be a power of two.
    ///
    pub const SCHEDULER_FREQ: usize = 128;
}

//==================================================================================================
// User Memory Layout
//==================================================================================================

pub mod memory_layout {
    use crate::mm::VirtualAddress;

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
    pub const USER_BASE: VirtualAddress = VirtualAddress::new(0x40000000);

    ///
    /// # Description
    ///
    /// End address of user space.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_END: VirtualAddress = VirtualAddress::new(0xc0000000);

    ///
    /// # Description
    ///
    /// Base address of user stack.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_STACK_BASE: VirtualAddress = VirtualAddress::new(0xc0000000);
}
