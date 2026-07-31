// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::ipc::VmIoRequest;

//==================================================================================================
// Global State
//==================================================================================================

static mut REQUEST: VmIoRequest = VmIoRequest::new(0, 1, false, 0);

//==================================================================================================
// Helpers
//==================================================================================================

#[inline(always)]
fn synchronize() {
    unsafe {
        core::arch::asm!("dsb sy", options(nostack, preserves_flags));
    }
}

#[inline(always)]
unsafe fn submit(port: u16, width: u8, write: bool, value: u32) -> u32 {
    let request: *mut VmIoRequest = &raw mut REQUEST;
    core::ptr::write_volatile(request, VmIoRequest::new(port, width, write, value));

    // Publish the normal-memory request before ringing the device-memory doorbell.
    synchronize();
    let doorbell: *mut usize = ::config::aarch64::DEFAULT_MMIO_PMIO_DOORBELL as *mut usize;
    core::ptr::write_volatile(doorbell, request as usize);

    // Observe the host response only after the doorbell transaction has completed.
    synchronize();
    core::ptr::read_volatile(request).value()
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Writes an 8-bit value to an emulated I/O port.
pub unsafe fn out8(port: u16, val: u8) {
    let _ = submit(port, 1, true, u32::from(val));
}

/// Writes a 16-bit value to an emulated I/O port.
pub unsafe fn out16(port: u16, val: u16) {
    let _ = submit(port, 2, true, u32::from(val));
}

/// Writes a 32-bit value to an emulated I/O port.
pub unsafe fn out32(port: u16, val: u32) {
    let _ = submit(port, 4, true, val);
}

/// Reads an 8-bit value from an emulated I/O port.
pub unsafe fn in8(port: u16) -> u8 {
    submit(port, 1, false, 0) as u8
}

/// Reads a 16-bit value from an emulated I/O port.
pub unsafe fn in16(port: u16) -> u16 {
    submit(port, 2, false, 0) as u16
}

/// Reads a 32-bit value from an emulated I/O port.
pub unsafe fn in32(port: u16) -> u32 {
    submit(port, 4, false, 0)
}

/// Serializes consecutive emulated I/O operations.
#[inline]
pub fn wait() {
    synchronize();
}
