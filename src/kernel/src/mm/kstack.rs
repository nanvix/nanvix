// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::PageAligned,
    mm::{
        KernelPage,
        VirtMemoryManager,
    },
};
use ::alloc::vec::Vec;
use ::sys::{
    arch::mem::PAGE_ALIGNMENT,
    error::Error,
    mm::{
        Address,
        VirtualAddress,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a kernel stack.
///
pub struct KernelStack {
    kpages: Vec<KernelPage>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl KernelStack {
    ///
    /// # Description
    ///
    /// Instantiates a new kernel stack.
    ///
    /// # Parameters
    ///
    /// - `mm`: A reference to the virtual memory manager.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the new kernel stack. Upon failure, an error is returned.
    ///
    #[allow(dead_code)] // TODO: Remove this attribute once the function is used.
    pub fn new(mm: &mut VirtMemoryManager) -> Result<Self, Error> {
        let kpages: Vec<KernelPage> =
            mm.alloc_kpages(true, config::kernel::KSTACK_SIZE / ::sys::arch::mem::PAGE_SIZE)?;

        Ok(Self { kpages })
    }

    ///
    /// # Description
    ///
    /// Returns the top address of the target kernel stack.
    ///
    /// # Returns
    ///
    /// The top address of the target kernel stack.
    ///
    /// # Notes
    ///
    /// The top address of the kernel stack is the address of the first byte after the kernel stack.
    ///
    #[allow(dead_code)] // TODO: Remove this attribute once the function is used.
    pub fn top(&self) -> PageAligned<VirtualAddress> {
        let base: usize = self.kpages[0].base().into_raw_value();
        let size: usize = config::kernel::KSTACK_SIZE;
        // SAFETY: The following call to unwrap is safe because the base address of the kernel stack
        // and the size of the kernel stack are both page aligned.
        debug_assert!(::sys::mm::is_aligned(base, PAGE_ALIGNMENT));
        debug_assert!(::sys::mm::is_aligned(size, PAGE_ALIGNMENT));
        PageAligned::from_raw_value(base + size).unwrap()
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl Drop for KernelStack {
    fn drop(&mut self) {
        while let Some(kpage) = self.kpages.pop() {
            drop(kpage);
        }
    }
}
