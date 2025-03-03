// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// User Memory Layout
//==================================================================================================
pub mod memory_layout {
    use crate::mm::VirtualAddress;
    use config::memory_layout::*;
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
    /// Base address of user heap.
    ///
    /// # Notes
    ///
    /// - This should be aligned to page and page table boundaries.
    ///
    pub const USER_HEAP_BASE: VirtualAddress = VirtualAddress::new(USER_HEAP_BASE_RAW);
}
