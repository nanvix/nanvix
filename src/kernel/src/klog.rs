// Copyright(c) 2dThe Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::hal::platform;
use ::core::{
    fmt,
    fmt::Write,
};

//==================================================================================================
// Structures
//==================================================================================================

/// Kernel log device.
pub struct Klog;

//==================================================================================================
// Enumerations
//==================================================================================================

/// Kernel log levels.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum KlogLevel {
    Panic,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum log level.
pub const MAX_LEVEL: KlogLevel = if cfg!(feature = "trace") {
    KlogLevel::Trace
} else if cfg!(feature = "debug") {
    KlogLevel::Debug
} else if cfg!(feature = "info") {
    KlogLevel::Info
} else if cfg!(feature = "warn") {
    KlogLevel::Warn
} else if cfg!(feature = "error") {
    KlogLevel::Error
} else {
    KlogLevel::Panic
};

//==================================================================================================
// Implementations
//==================================================================================================

impl Klog {
    ///
    /// # Description
    ///
    /// Instantiates a kernel log with a given tag and level.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the kernel log.
    /// - `level`: Level of the kernel log.
    ///
    /// # Returns
    ///
    /// A kernel log instance.
    ///
    pub fn get(tag: &str, level: KlogLevel) -> Self {
        let mut ret: Self = Self;
        let _ = write!(&mut ret, "[{:?}][{}] ", level, tag);
        ret
    }
}

impl Drop for Klog {
    fn drop(&mut self) {
        let _ = writeln!(self);
    }
}

impl fmt::Write for Klog {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe { puts(s) };
        Ok(())
    }
}

impl core::fmt::Debug for KlogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KlogLevel::Trace => write!(f, "TRACE"),
            KlogLevel::Debug => write!(f, "DEBUG"),
            KlogLevel::Info => write!(f, "INFO"),
            KlogLevel::Warn => write!(f, "WARN"),
            KlogLevel::Error => write!(f, "ERROR"),
            KlogLevel::Panic => write!(f, "PANIC"),
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes the string `s` to the platform's standard debug device.
///
/// # Parameters
///
/// - `s`: String to write.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
///
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn puts(s: &str) {
    // Write each byte of the string to the standard output device.
    for b in s.bytes() {
        platform::putb(b);
    }
}
