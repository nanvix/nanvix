// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::dbg;
use ::alloc::boxed::Box;

//==================================================================================================
// Traits
//==================================================================================================

pub trait Stdout {
    fn puts(&mut self, s: &str);
}

//==================================================================================================
// Global Variables
//==================================================================================================

static mut STDOUT: Option<Box<dyn Stdout>> = None;

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub unsafe fn init(debugger: Box<dyn Stdout>) {
    STDOUT = Some(debugger);
}

///
/// # Description
///
/// Writes a string to the platform's standard output device.
///
/// # Parameters
///
/// - `s`: The string to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn puts(s: &str) {
    if let Some(debugger) = &mut STDOUT {
        debugger.puts(s);
    } else {
        dbg::puts(s);
    }
}
