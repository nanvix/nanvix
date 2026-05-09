// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Kernel frame type.
//!
//! A [`KernelFrame`] represents a single page-sized physical frame allocated for kernel use.
//! It provides `Deref`/`DerefMut` for direct kernel access (identity-mapped) and frees the
//! underlying frame on drop via the global frame allocator.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::mem::FrameAddress;
#[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
use crate::hal::mem::{
    Address,
    PageAligned,
    PhysicalAddress,
};
use ::arch::mem;
use ::core::ops::{
    Deref,
    DerefMut,
};
use ::sys::error::Error;

//==================================================================================================
// Kernel Frame
//==================================================================================================

/// A type that represents a kernel frame.
#[derive(Debug)]
pub struct KernelFrame {
    /// Frame address.
    base: FrameAddress,
}

impl KernelFrame {
    ///
    /// # Description
    ///
    /// Instantiates a kernel frame.
    ///
    /// # Parameters
    ///
    /// - `base`: Frame address.
    ///
    /// # Returns
    ///
    /// Upon success, a kernel frame is returned. Upon failure, an error is returned instead.
    ///
    pub(super) fn new(base: FrameAddress) -> Result<Self, Error> {
        // Ensure the frame is identity-mapped in the kernel address space so that
        // Deref/DerefMut can safely access it. On microvm this lazily installs a page
        // table entry if needed (page tables come from a BSS pool, so no recursive frame
        // allocation occurs). On hyperlight (host-bootstrapped VAS) all memory is already
        // mapped, so this is compiled out.
        #[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
        {
            let phys_addr: PageAligned<PhysicalAddress> =
                PageAligned::from_raw_value(base.into_raw_value()).map_err(|e| {
                    error!("KernelFrame::new(): frame base is not page-aligned: {:?}", e);
                    e
                })?;
            crate::mm::virt::identity_map_page(phys_addr).map_err(|e| {
                error!("KernelFrame::new(): failed to identity-map frame: {:?}", e);
                e
            })?;
        }
        Ok(Self { base })
    }

    ///
    /// # Description
    ///
    /// Returns the base address of the target kernel frame.
    ///
    /// # Returns
    ///
    /// The base address of the target kernel frame.
    ///
    pub fn base(&self) -> FrameAddress {
        self.base
    }

    ///
    /// # Description
    ///
    /// Clears the target kernel frame.
    ///
    /// Uses the identity-map / no-identity-map `memset` backend so that the
    /// write runs in the kernel address space.  This avoids a page fault
    /// when the current CR3 points to a user page directory that lacks the
    /// PDE for this frame's physical address.
    ///
    pub fn clear(&mut self) {
        let base: *mut u8 = self.accessible_address() as *mut u8;
        crate::mm::virt::memset(base, 0, mem::PAGE_SIZE)
            .expect("KernelFrame::clear(): memset failed");
    }

    /// Returns the virtual address at which this frame can be accessed by the kernel.
    /// On microvm (identity-mapped): GVA == GPA.
    /// On hyperlight: translates GPA→GVA for scratch-region frames.
    #[inline]
    fn accessible_address(&self) -> usize {
        #[cfg(feature = "platform-root-virtual-address-space-bootstrap")]
        {
            crate::hal::platform::gpa_to_gva(self.base.into_raw_value())
        }
        #[cfg(not(feature = "platform-root-virtual-address-space-bootstrap"))]
        {
            self.base.into_raw_value()
        }
    }
}

impl Deref for KernelFrame {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        let ptr: usize = self.accessible_address();
        unsafe { core::slice::from_raw_parts(ptr as *const u8, mem::PAGE_SIZE) }
    }
}

impl DerefMut for KernelFrame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr: usize = self.accessible_address();
        unsafe { core::slice::from_raw_parts_mut(ptr as *mut u8, mem::PAGE_SIZE) }
    }
}

impl Drop for KernelFrame {
    fn drop(&mut self) {
        if let Err(e) = super::frame::free(self.base) {
            error!("failed to free kernel frame: {:?}", e);
        }
    }
}
