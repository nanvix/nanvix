// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the 8-bit value `val` to the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
pub unsafe fn out8(port: u16, val: u8) {
    arch::asm!("out dx, al", in("dx") port, in("al") val, options(preserves_flags, nomem, nostack));
}

///
/// # Description
///
/// Writes the 16-bit value `val` to the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
pub unsafe fn out16(port: u16, val: u16) {
    arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(preserves_flags, nomem, nostack));
}

///
/// # Description
///
/// Writes the 32-bit value `val` to the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
#[allow(dead_code)]
pub unsafe fn out32(port: u16, val: u32) {
    arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(preserves_flags, nomem, nostack));
}

///
/// # Description
///
/// Reads an 8-bit value from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
pub unsafe fn in8(port: u16) -> u8 {
    let ret: u8;
    arch::asm!("in al, dx", out("al") ret, in("dx") port, options(preserves_flags, nomem, nostack));
    ret
}

///
/// # Description
///
/// Reads a 16-bit value from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
pub unsafe fn in16(port: u16) -> u16 {
    let ret: u16;
    arch::asm!("in ax, dx", out("ax") ret, in("dx") port, options(preserves_flags, nomem, nostack));
    ret
}

///
/// # Description
///
/// Reads a 32-bit value from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe because it performs raw I/O port access.
///
#[allow(dead_code)]
pub unsafe fn in32(port: u16) -> u32 {
    let ret: u32;
    arch::asm!("in eax, dx", out("eax") ret, in("dx") port, options(preserves_flags, nomem, nostack));
    ret
}

///
/// # Description
///
/// Waits for an I/O operation to complete.
///
pub fn wait() {
    #[cfg(feature = "pc")]
    unsafe {
        out8(0x80, 0)
    };
}
