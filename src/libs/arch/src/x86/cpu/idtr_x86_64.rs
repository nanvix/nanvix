// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Structures
//==================================================================================================

/// Interrupt descriptor table pointer (IDTR) for 64-bit x86_64.
#[repr(C, packed)]
pub struct Idtr {
    pub size: u16, // IDT size.
    pub ptr: u64,  // IDT virtual address.
}
::static_assert::assert_eq_size!(Idtr, 10);

impl Idtr {
    /// Initializes an IDT register.
    pub unsafe fn init(&mut self, ptr: u64, size: u16) {
        self.size = size - 1;
        self.ptr = ptr;
    }

    /// Loads the IDT.
    pub unsafe fn load(&self) {
        arch::asm!("lidt (%rax)", in("rax") self, options(nostack, nomem, att_syntax));
    }
}
