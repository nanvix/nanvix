// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    fmt,
    fmt::Write,
};

//==================================================================================================
// Macros
//==================================================================================================

#[macro_export]
macro_rules! trace {
    (target: $target:expr, $($arg:tt)+) => ({
        if $crate::MAX_LEVEL >= $crate::LogLevel::Trace {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get($target, $crate::LogLevel::Trace),
                $($arg)+
            );
        }
    });
    ( $($arg:tt)+ ) => ({
        if $crate::MAX_LEVEL >= $crate::LogLevel::Trace {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Trace),
                $($arg)+
            );
        }
    })
}

#[macro_export]
macro_rules! debug{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Debug{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Debug),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! info{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Info {
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Info),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! warn{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Warn{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Warn),
                $($arg)*
            );
        }
    })
}

#[macro_export]
macro_rules! error{
    ( $($arg:tt)* ) => ({
		if $crate::MAX_LEVEL >= $crate::LogLevel::Error{
            use core::fmt::Write;
            let _ = writeln!(
                &mut $crate::Logger::get(module_path!(), $crate::LogLevel::Error),
                $($arg)*
            );
        }
    })
}

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum log level.
pub const MAX_LEVEL: LogLevel = if cfg!(feature = "trace") {
    LogLevel::Trace
} else if cfg!(feature = "debug") {
    LogLevel::Debug
} else if cfg!(feature = "info") {
    LogLevel::Info
} else if cfg!(feature = "warn") {
    LogLevel::Warn
} else if cfg!(feature = "error") {
    LogLevel::Error
} else {
    LogLevel::Panic
};

//==================================================================================================
// Structures
//==================================================================================================

/// The `Logger` struct acts as a formatter and output handler for log messages within the logging
/// system.
///
/// It is used internally by the logging macros (`trace!`, `debug!`, `info!`, `warn!`, `error!`) to
/// format log messages with the appropriate log level and module path, and to output them to the
/// system's debug interface.
///
/// The `Logger` implements the [`core::fmt::Write`] trait, allowing it to be used with Rust's
/// formatting macros.  The [`Logger::get`] method is used to create a new `Logger` instance with a
/// given tag and log level, which prefixes each log message with the log level and module path.
///
/// This struct is not intended to be used directly by end users; instead, use the provided logging
/// macros.
///
pub struct Logger;

/// Log levels.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Panic,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl core::fmt::Debug for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Panic => write!(f, "PANIC"),
        }
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let slice: &[u8] = s.as_bytes();
        let _ = ::sys::kcall::debug::debug(slice.as_ptr(), slice.len());
        Ok(())
    }
}

impl Logger {
    ///
    /// # Description
    ///
    /// Creates a new `Logger` instance and writes a log prefix with the specified tag and log level.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag to associate with the log message (typically the module path).
    /// * `level` - The log level for the message.
    ///
    /// # Returns
    ///
    /// A `Logger` instance with the log prefix written.
    ///
    pub fn get(tag: &str, level: LogLevel) -> Self {
        let mut ret: Self = Self;
        let _ = write!(&mut ret, "[{level:?}][{tag}] ");
        ret
    }
}
