// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

/// Interrupt descriptor table register (IDTR) for x86_64.
/// In long mode, the base address is 64 bits.
#[repr(C, packed)]
pub struct Idtr {
    /// IDT size minus one.
    pub size: u16,
    /// 64-bit virtual address of the IDT.
    pub ptr: u64,
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
        core::arch::asm!("lidt [{}]", in(reg) self, options(nostack, nomem));
    }
}
