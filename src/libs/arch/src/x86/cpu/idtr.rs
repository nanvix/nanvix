// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Structures
//==================================================================================================

#[cfg(target_arch = "x86")]
/// Interrupt descriptor table pointer (IDTR).
#[repr(C, packed)]
pub struct Idtr {
    pub size: u16, // IDT size.
    pub ptr: u32,  // IDT virtual address.
}
#[cfg(target_arch = "x86")]
::static_assert::assert_eq_size!(Idtr, 6);

#[cfg(target_arch = "x86")]
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

#[cfg(target_arch = "x86_64")]
/// Interrupt descriptor table pointer (IDTR) for 64-bit x86_64.
#[repr(C, packed)]
pub struct Idtr {
    pub size: u16, // IDT size.
    pub ptr: u64,  // IDT virtual address.
}
#[cfg(target_arch = "x86_64")]
::static_assert::assert_eq_size!(Idtr, 10);

#[cfg(target_arch = "x86_64")]
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
