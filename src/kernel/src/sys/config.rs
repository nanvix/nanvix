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
    /// User stack size (in bytes).
    ///
    /// # Notes
    ///
    /// - This size should be a multiple of a page size.
    ///
    pub const USTACK_SIZE: usize = 16 * 4 * constants::KILOBYTE;

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

    ///
    /// # Description
    ///
    /// Maximum number of messages that can be buffered by the kernel.
    ///
    /// # Notes
    ///
    /// - When this threshold is reached, inter-kernel communication is blocked.
    /// - This value should be set according to the amount of memory available in the kernel heap.
    ///
    pub const MAX_IKC_MESSAGES: usize = 128;

    ///
    /// # Description
    ///
    /// Size of an IPC message.
    ///
    /// # Notes
    ///
    /// - The value of this function has direct impact on IPC performance.
    /// - The default value is set to match the size of a cache line in x86 processors.
    ///
    pub const IPC_MESSAGE_SIZE: usize = 64;
}

//==================================================================================================
// User Memory Layout
//==================================================================================================

pub mod memory_layout {
    use crate::mm::VirtualAddress;

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_BASE`], which can be used in constant-value expressions.
    ///
    pub const KPOLL_BASE_RAW: usize = 0x00400000;

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
    pub const KPOOL_BASE: VirtualAddress = VirtualAddress::new(KPOLL_BASE_RAW);

    ///
    /// # Description
    ///
    /// Provides the raw value for [`KPOOL_END`], which can be used in constant-value expressions.
    ///
    pub const USER_BASE_RAW: usize = 0x40000000;

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
    /// Provides the raw value for [`USER_END`], which can be used in constant-value expressions.
    ///
    pub const USER_END_RAW: usize = 0xf0000000;

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
    /// Provides the raw value for [`USER_HEAP_BASE`], which can be used in constant-value expressions.
    ///
    pub const USER_HEAP_BASE_RAW: usize = 0xa0000000;

    ///
    /// # Description
    ///
    /// Base address of user heap.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_HEAP_BASE: VirtualAddress = VirtualAddress::new(USER_HEAP_BASE_RAW);

    ///
    /// # Description
    ///
    /// Base address of user stack.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_STACK_BASE: VirtualAddress = USER_END;
}
