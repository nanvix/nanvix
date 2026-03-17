// Copyright(c) The Maintainers of Nanvix.
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
    /// Instantiates a kernel log with a given tag, level, and function name.
    ///
    /// # Parameters
    ///
    /// - `tag`: Tag of the kernel log (module path).
    /// - `level`: Level of the kernel log.
    /// - `function_name`: Name of the function from which the log is called.
    ///
    /// # Returns
    ///
    /// A kernel log instance.
    ///
    pub fn get(tag: &str, level: KlogLevel, function_name: &str) -> Self {
        let mut ret: Self = Self;

        // Extract just the module name from the full module path.
        let module_name: &str = tag.split("::").last().unwrap_or(tag);

        let _ = write!(&mut ret, "[{level:?}][{module_name}] {function_name}(): ");
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
        // SAFETY: the standard output device is present, initialized, and accessed
        // exclusively from a single core with interrupts disabled.
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
/// Writes the string `s` directly to the platform's standard debug device, bypassing the kernel log
/// buffer. This is used internally by the buffer's flush path.
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
unsafe fn raw_puts(s: &str) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "puts")] {
            platform::puts(s);
        } else if #[cfg(feature = "putb")] {
            // Write each byte of the string to the standard output device.
            for b in s.bytes() {
                platform::putb(b);
            }
        }
    }
}

///
/// # Description
///
/// Writes the string `s` to the kernel log buffer. The buffered data is flushed to the platform's
/// standard debug device in bulk when the buffer is full or when [`flush()`] is called.
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
pub unsafe fn puts(_s: &str) {
    #[cfg(not(feature = "smp"))]
    {
        // Logging is disabled when the buffer size is zero.
        if BUFFER_SIZE == 0 {
            return;
        }

        KLOG_BUFFER.get().append(_s);
    }
}

///
/// # Description
///
/// Flushes the kernel log buffer, writing all buffered data to the platform's standard debug device
/// in bulk. This should be called periodically (e.g., from the kernel call handler loop) and before
/// shutdown to ensure no output is lost.
///
/// # Safety
///
/// This function is unsafe for multiple reasons:
///
/// - It assumes that the standard output device is present.
/// - It assumes that the standard output device was properly initialized.
/// - It does not prevent concurrent access to the standard output device.
///
pub unsafe fn flush() {
    #[cfg(not(feature = "smp"))]
    {
        // Logging is disabled when the buffer size is zero.
        if BUFFER_SIZE == 0 {
            return;
        }

        KLOG_BUFFER.get().flush();
    }
}

//==================================================================================================
// Kernel Log Buffer
//==================================================================================================

// The kernel log buffer infrastructure is only available in single-core builds. Under SMP, all
// output is written directly to the platform device (no buffering).
#[cfg(not(feature = "smp"))]
mod buffer {
    use super::*;
    use ::core::cell::UnsafeCell;

    /// Size of the kernel log buffer in bytes.
    pub const BUFFER_SIZE: usize = config::kernel::KLOG_BUFFER_SIZE;

    /// Global kernel log buffer.
    ///
    /// Wrapped in [`SyncUnsafeCell`] to allow safe static storage on a single-core system.
    pub static KLOG_BUFFER: SyncUnsafeCell<KlogBuffer> = SyncUnsafeCell::new(KlogBuffer::new());

    /// A thin wrapper around [`UnsafeCell`] that implements [`Sync`].
    ///
    /// # Safety
    ///
    /// This is sound because the Nanvix kernel runs on a single core with interrupts disabled
    /// during all access to the wrapped value, so no concurrent access can occur.
    pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

    impl<T> SyncUnsafeCell<T> {
        pub const fn new(value: T) -> Self {
            Self(UnsafeCell::new(value))
        }

        /// Returns a mutable reference to the inner value.
        ///
        /// # Safety
        ///
        /// The caller must ensure that no other reference to the inner value exists.
        #[allow(clippy::mut_from_ref)]
        pub unsafe fn get(&self) -> &mut T {
            &mut *self.0.get()
        }
    }

    // SAFETY: the kernel is single-core with interrupts disabled during access.
    unsafe impl<T> Sync for SyncUnsafeCell<T> {}

    /// A fixed-size buffer that batches kernel log output before flushing to the platform device.
    ///
    /// All kernel log output (from logging macros and the debug kernel call) is appended to this
    /// buffer instead of being written directly to the output device. The buffer is flushed in bulk
    /// when it is full or when an explicit flush is requested, reducing per-character I/O overhead.
    pub struct KlogBuffer {
        /// Underlying storage.
        data: [u8; BUFFER_SIZE],
        /// Number of valid bytes currently stored in `data`.
        len: usize,
    }

    impl KlogBuffer {
        /// Creates a new, empty kernel log buffer.
        pub const fn new() -> Self {
            Self {
                data: [0; BUFFER_SIZE],
                len: 0,
            }
        }

        ///
        /// # Description
        ///
        /// Appends the string `s` to the buffer. If the incoming data does not fit, the buffer is
        /// flushed first (force-flush policy) before appending. Messages larger than the entire
        /// buffer are flushed and then written directly to the platform output.
        ///
        /// # Parameters
        ///
        /// - `s`: String to append.
        ///
        /// # Safety
        ///
        /// This function is unsafe for multiple reasons:
        ///
        /// - It assumes that the standard output device is present.
        /// - It assumes that the standard output device was properly initialized.
        /// - It does not prevent concurrent access to the standard output device.
        ///
        pub unsafe fn append(&mut self, s: &str) {
            let bytes: &[u8] = s.as_bytes();

            // If the message is larger than the entire buffer, flush current contents and write
            // directly to the platform output to avoid losing data.
            if bytes.len() > BUFFER_SIZE {
                self.flush();
                raw_puts(s);
                return;
            }

            // If the message does not fit in the remaining space, flush first.
            if self.len + bytes.len() > BUFFER_SIZE {
                self.flush();
            }

            // Append to buffer.
            self.data[self.len..self.len + bytes.len()].copy_from_slice(bytes);
            self.len += bytes.len();

            // If the buffer is now exactly full, flush immediately to honor the documented policy.
            if self.len == BUFFER_SIZE {
                self.flush();
            }
        }

        ///
        /// # Description
        ///
        /// Flushes all buffered data to the platform's standard debug device and resets the buffer.
        ///
        /// # Safety
        ///
        /// This function is unsafe for multiple reasons:
        ///
        /// - It assumes that the standard output device is present.
        /// - It assumes that the standard output device was properly initialized.
        /// - It does not prevent concurrent access to the standard output device.
        ///
        pub unsafe fn flush(&mut self) {
            // No data to flush.
            if self.len == 0 {
                return;
            }

            // SAFETY: the buffer only contains bytes from valid UTF-8 strings appended via
            // `append()`.
            let s: &str = core::str::from_utf8_unchecked(&self.data[..self.len]);
            raw_puts(s);
            self.len = 0;
        }
    }
}

#[cfg(not(feature = "smp"))]
use buffer::*;
