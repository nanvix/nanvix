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
use ::arch::mem::PAGE_ALIGNMENT;
#[cfg(debug_assertions)]
use ::config::kernel::KSTACK_GUARD_PATTERN;
use ::core::fmt;
#[cfg(debug_assertions)]
use ::sys::error::ErrorCode;
use ::sys::{
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
    /// Instantiates a new kernel stack with a guard page watermark at the bottom.
    ///
    /// The bottom page of the kernel stack is filled with a known watermark pattern. If a stack
    /// overflow occurs, the watermark will be corrupted, allowing runtime detection via
    /// [`KernelStack::check_guard_watermark`].
    ///
    /// # Parameters
    ///
    /// - `mm`: A reference to the virtual memory manager.
    ///
    /// # Returns
    ///
    /// Upon success, the function returns the new kernel stack. Upon failure, an error is returned.
    ///
    pub fn new(mm: &mut VirtMemoryManager) -> Result<Self, Error> {
        let kpages: Vec<KernelPage> =
            mm.alloc_kpages(true, config::kernel::KSTACK_SIZE / ::arch::mem::PAGE_SIZE)?;

        let stack: Self = Self { kpages };

        // Fill the bottom page of the stack with the watermark pattern.
        #[cfg(debug_assertions)]
        stack.fill_guard_watermark();

        Ok(stack)
    }

    ///
    /// # Description
    ///
    /// Returns the size of the target kernel stack.
    ///
    /// # Returns
    ///
    /// The size of the target kernel stack.
    ///
    fn size(&self) -> usize {
        config::kernel::KSTACK_SIZE
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel stack.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel stack.
    ///
    /// # Notes
    ///
    /// As stacks grow downwards, the base address is the highest address of the stack.
    ///
    fn base(&self) -> PageAligned<VirtualAddress> {
        PageAligned::from_raw_value(self.kpages[0].base().into_raw_value()).unwrap()
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
    pub fn top(&self) -> PageAligned<VirtualAddress> {
        let base: usize = self.kpages[0].base().into_raw_value();
        let size: usize = config::kernel::KSTACK_SIZE;
        // SAFETY: The following call to unwrap is safe because the base address of the kernel stack
        // and the size of the kernel stack are both page aligned.
        debug_assert!(::sys::mm::is_aligned(base, PAGE_ALIGNMENT));
        debug_assert!(::sys::mm::is_aligned(size, PAGE_ALIGNMENT));
        PageAligned::from_raw_value(base + size).unwrap()
    }

    ///
    /// # Description
    ///
    /// Fills the bottom page of the kernel stack with the watermark pattern.
    ///
    #[cfg(debug_assertions)]
    fn fill_guard_watermark(&self) {
        let guard_base: usize = self.kpages[0].base().into_raw_value();
        let word_count: usize = ::arch::mem::PAGE_SIZE / ::core::mem::size_of::<u32>();
        let guard_ptr: *mut u32 = guard_base as *mut u32;
        for i in 0..word_count {
            // SAFETY: The guard page is within allocated kernel memory and no other references
            // to this memory exist at this point, so writing through a raw pointer derived from
            // the page base address is sound.
            unsafe {
                guard_ptr.add(i).write_volatile(KSTACK_GUARD_PATTERN);
            }
        }
    }

    ///
    /// # Description
    ///
    /// Checks the guard watermark at the bottom page of the kernel stack for corruption.
    ///
    /// # Returns
    ///
    /// Upon success (watermark intact), `Ok(())` is returned. Upon failure (watermark corrupted),
    /// an error is returned.
    ///
    pub fn check_guard_watermark(&self) -> Result<(), Error> {
        cfg_if::cfg_if! {
            if #[cfg(debug_assertions)] {
                check_guard_page(self.kpages[0].base().into_raw_value())
            } else {
                Ok(())
            }
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Checks a guard page at the given base address for watermark pattern corruption.
///
/// # Parameters
///
/// - `guard_base`: The base address of the guard page to check.
///
/// # Returns
///
/// Upon success (watermark intact), `Ok(())` is returned. Upon failure (watermark corrupted),
/// an error is returned.
///
#[cfg(debug_assertions)]
fn check_guard_page(guard_base: usize) -> Result<(), Error> {
    let word_count: usize = ::arch::mem::PAGE_SIZE / ::core::mem::size_of::<u32>();
    let guard_ptr: *const u32 = guard_base as *const u32;
    for i in 0..word_count {
        // SAFETY: The caller guarantees that the guard page at `guard_base` is within valid
        // kernel memory (either allocated kernel pages or the BSS section).
        let val: u32 = unsafe { guard_ptr.add(i).read_volatile() };
        if val != KSTACK_GUARD_PATTERN {
            let reason: &str = "kernel stack guard watermark corrupted (possible stack overflow)";
            error!("{}", reason);
            return Err(Error::new(ErrorCode::BadAddress, reason));
        }
    }
    Ok(())
}

///
/// # Description
///
/// Checks the boot stack guard watermark for corruption. The boot stack guard page is filled with a
/// known pattern during early boot (see `start.S`). If a stack overflow has occurred, some or all
/// of the guard page words will have been overwritten.
///
/// # Returns
///
/// Upon success (watermark intact), `Ok(())` is returned. Upon failure (watermark corrupted),
/// an error is returned.
///
/// # Notes
///
/// This check runs after `mm::init()` completes. If the stack overflowed during `hal::init()` or
/// `mm::init()`, the corruption may have been overwritten by later stack frames shrinking. The
/// check is best-effort and may not detect all early-boot overflows.
///
pub fn check_boot_stack_guard() -> Result<(), Error> {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            unsafe extern "C" {
                static kstack_guard: u8;
            }
            let guard_base: usize = unsafe { &kstack_guard as *const u8 as usize };
            check_guard_page(guard_base)
        } else {
            Ok(())
        }
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Debug for KernelStack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "KernelStack {{ base: {:?}, top: {:?}, size={:?} }}",
            self.base(),
            self.top(),
            self.size()
        )
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        debug!("{:?}", &self);
        while let Some(kpage) = self.kpages.pop() {
            drop(kpage);
        }
    }
}
