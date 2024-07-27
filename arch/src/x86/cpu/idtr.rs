// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::arch;

//==================================================================================================
// Structures
//==================================================================================================

/// Interrupt descriptor table pointer (IDTR).
#[repr(C, packed)]
pub struct Idtr {
    pub size: u16, // IDT size.
    pub ptr: u32,  // IDT virtual address.
}
static_assert_size!(Idtr, 6);

impl Idtr {
    /// Initializes an IDT register.
    pub unsafe fn init(&mut self, ptr: u32, size: u16) {
        self.size = size - 1;
        self.ptr = ptr;
    }

    /// Loads the IDT.
    pub unsafe fn load(&self) {
        arch::asm!("lidt (%eax)", in("eax") self, options(nostack, nomem, att_syntax));
    }
}
