// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::PageAligned,
    mm::{
        phys::KernelFrame,
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
// Constants
//==================================================================================================

#[cfg(feature = "exception-stack-guard")]
use ::arch::cpu::excp::Exception;

//==================================================================================================
// Global Variables
//==================================================================================================

/// Dynamic stack overflow guard threshold, read by the assembly `excp_stack_guard_check` macro.
///
/// Holds the lowest safe ESP value for the currently-active kernel stack. Updated on every context
/// switch and at boot. A value of 0 disables the guard check.
///
/// TODO (#1665): this is a single global, so it is only correct on uniprocessor builds. For SMP,
/// replace with a per-core variable (e.g., indexed by APIC ID or stored in per-core data).
#[cfg(feature = "exception-stack-guard")]
#[unsafe(no_mangle)]
pub static EXCP_STACK_GUARD: ::core::sync::atomic::AtomicU32 =
    ::core::sync::atomic::AtomicU32::new(0);

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
    /// Lowest safe ESP value for this stack (base + CONTEXT_HW_SIZE).
    #[cfg(feature = "exception-stack-guard")]
    guard_threshold: u32,
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
        let count: usize = config::kernel::KSTACK_SIZE / ::arch::mem::PAGE_SIZE;
        let mut kframes: Vec<KernelFrame> = crate::mm::try_vec_with_capacity(count)?;
        mm.alloc_kpages(true, count, &mut kframes)?;
        let kpages: Vec<KernelPage> = kframes.into_iter().map(KernelPage::new).collect();

        #[cfg(feature = "exception-stack-guard")]
        let guard_threshold: u32 =
            (kpages[0].base().into_raw_value() + Exception::CONTEXT_HW_SIZE) as u32;

        let stack: Self = Self {
            kpages,
            #[cfg(feature = "exception-stack-guard")]
            guard_threshold,
        };

        // Fill the bottom page of the stack with the watermark pattern.
        #[cfg(debug_assertions)]
        stack.fill_guard_watermark();

        Ok(stack)
    }

    ///
    /// # Description
    ///
    /// Returns the guard threshold for this kernel stack. This is the lowest safe ESP value:
    /// the stack base address plus the maximum hardware-pushed exception frame size.
    ///
    /// # Returns
    ///
    /// The guard threshold value.
    ///
    #[cfg(feature = "exception-stack-guard")]
    pub fn guard_threshold(&self) -> u32 {
        self.guard_threshold
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
    pub(crate) fn size(&self) -> usize {
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
    pub(crate) fn base(&self) -> PageAligned<VirtualAddress> {
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
            // SAFETY: The guard page is within allocated kernel memory and no other
            // references to this memory exist at this point, so writing through a
            // raw pointer derived from the page base address is sound.
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
#[cfg(feature = "exception-stack-guard")]
pub fn check_boot_stack_guard() -> Result<(), Error> {
    cfg_if::cfg_if! {
        if #[cfg(debug_assertions)] {
            let guard_base: usize = crate::hal::platform::get_kstack_guard_base();
            check_guard_page(guard_base)
        } else {
            Ok(())
        }
    }
}

///
/// # Description
///
/// Sets the active stack overflow guard threshold. Called on every context switch to update the
/// assembly-level guard to the currently-active kernel stack.
///
/// # Parameters
///
/// - `threshold`: The guard threshold (lowest safe ESP) for the new active stack.
///
#[cfg(feature = "exception-stack-guard")]
pub fn set_active_guard(threshold: u32) {
    EXCP_STACK_GUARD.store(threshold, core::sync::atomic::Ordering::Release);
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

        // If this stack was the active one, clear the guard so a stale threshold is never
        // checked against a freed stack region.
        #[cfg(feature = "exception-stack-guard")]
        let _ = EXCP_STACK_GUARD.compare_exchange(
            self.guard_threshold,
            0,
            core::sync::atomic::Ordering::Release,
            core::sync::atomic::Ordering::Relaxed,
        );

        while let Some(kpage) = self.kpages.pop() {
            drop(kpage);
        }
    }
}
