// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::arch;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the 8-bit value `val` to the I/O port `port`.
///
/// # Parameters
///
/// - `port`: I/O port to write to.
/// - `val`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether writing an 8-bit value to the specified port is allowed.
///
pub unsafe fn out8(port: u16, val: u8) {
    arch::asm!("out dx, al", in("dx") port, in("al") val, options(preserves_flags, nomem, nostack));
}

///
/// # Description
///
/// Writes the 16-bit value `val` to the I/O port `port`.
///
/// # Parameters
///
/// - `port`: I/O port to write to.
/// - `val`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether writing a 16-bit value to the specified port is allowed.
///
pub unsafe fn out16(port: u16, val: u16) {
    arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(preserves_flags, nomem, nostack));
}

///
/// # Description
///
/// Writes the 32-bit value `val` to the I/O port `port`.
///
/// # Parameters
///
/// - `port`: I/O port to write to.
/// - `val`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether writing a 32-bit value to the specified port is allowed.
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
/// # Parameters
///
/// - `port`: I/O port to read from.
///
/// # Returns
///
/// The 8-bit value read from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether reading an 8-bit value from the specified port is allowed.
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
/// # Parameters
///
/// - `port`: I/O port to read from.
///
/// # Returns
///
/// The 16-bit value read from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether reading a 16-bit value from the specified port is allowed.
///
pub unsafe fn in16(port: u16) -> u16 {
    let ret: u16;
    arch::asm!("in eax, dx", out("eax") ret, in("dx") port, options(preserves_flags, nomem, nostack));
    ret
}

///
/// # Description
///
/// Reads a 32-bit value from the I/O port `port`.
///
/// # Parameters
///
/// - `port`: I/O port to read from.
///
/// # Returns
///
/// The 32-bit value read from the I/O port `port`.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It emits inline assembly.
/// - It does not check wether the specified port is valid.
/// - It does not check wether reading a 32-bit value from the specified port is allowed.
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
    unsafe { out8(0x80, 0) };
}
