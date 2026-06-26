// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Schedules all modified in-core file data to be written to the underlying storage devices. Nanvix
/// does not maintain a global buffer cache that can be flushed atomically; durability is provided on
/// a per-descriptor basis through `fsync()`/`fdatasync()`. This call is therefore a no-op that exists
/// for source compatibility with POSIX applications.
///
#[trace_syscall]
#[unsafe(no_mangle)]
pub extern "C" fn sync() {}
