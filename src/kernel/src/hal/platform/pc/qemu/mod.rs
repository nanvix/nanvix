// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(feature = "qemu-isapc")]
mod isapc;

#[cfg(feature = "qemu-pc")]
mod pc;

//==================================================================================================
// Exports
//==================================================================================================

#[cfg(feature = "qemu-isapc")]
pub use isapc::shutdown;

#[cfg(feature = "qemu-pc")]
pub use pc::shutdown;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the 8-bit value `b` to the platform's standard output device.
///
/// # Parameters
///
/// - `b`: Value to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn putb(b: u8) {
    ::sys::arch::io::out8(::config::hal::DEFAULT_STDOUT_PORT, b);
}
