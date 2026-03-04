// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::x86_64::io;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the string `s` to the platform's standard output device.
///
/// # Safety
///
/// This function is unsafe because it assumes the UART device is present and initialized.
///
pub unsafe fn puts(s: &str) {
    for b in s.bytes() {
        putb(b);
    }
}

///
/// # Description
///
/// Writes the 8-bit value `b` to the platform's standard output device.
///
/// # Safety
///
/// This function is unsafe because it assumes the UART device is present and initialized.
///
unsafe fn putb(b: u8) {
    const UART_BASE: u16 = 0x3f8;
    const UART_THR: u16 = UART_BASE;
    const UART_LSR: u16 = UART_BASE + 5;
    const UART_LSR_THRE: u8 = 0x20;

    while (io::in8(UART_LSR) & UART_LSR_THRE) == 0 {}

    io::out8(UART_THR, b);
}
