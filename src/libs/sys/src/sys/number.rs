// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::num_enum::{
    FromPrimitive,
    IntoPrimitive,
};

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// An enumeration of kernel call numbers.
///
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, IntoPrimitive, FromPrimitive)]
pub enum KcallNumber {
    /// Debug.
    Debug = 0,
    /// Get process identifier.
    GetPid,
    /// Get thread identifier.
    GetTid,
    /// Terminate the calling process.
    Exit,
    /// Control capabilities.
    CapCtl,
    /// Resumes an interrupted process.
    Resume,
    /// Terminates a process.
    Terminate,
    /// Controls events.
    EventCtrl,
    /// Sends a message.
    Send,
    /// Receives a message.
    Recv,
    /// Map memory page.
    MemoryMap,
    /// Unmap memory page.
    MemoryUnmap,
    /// Controls a memory page.
    MemoryCtrl,
    /// Copies a memory page.
    MemoryCopy,
    /// Allocates a memory-mapped I/O region.
    AllocMmio,
    /// Releases a memory-mapped I/O region.
    FreeMmio,
    /// Allocates a port-mapped I/O port.
    AllocPmio,
    /// Frees a port-mapped I/O port.
    FreePmio,
    /// Reads a value from a port-mapped I/O port.
    ReadPmio,
    /// Writes a value to a port-mapped I/O port.
    WritePmio,
    /// Yields the processor.
    SchedulerYield,
    /// Create a new thread.
    CreateThread,
    /// Terminates the calling thread.
    ExitThread,
    /// Joins with a terminated thread.
    JoinThread,
    /// Locks a mutex.
    MutexLock,
    /// Unlocks a mutex.
    MutexUnlock,
    /// Signals a condition variable.
    CondSignal,
    /// Waits on a condition variable.
    CondWait,
    /// Gets the current system time.
    GetTime,
    /// Invalid kernel call.
    #[num_enum(default)]
    Invalid,
}
