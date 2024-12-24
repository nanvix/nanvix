// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Enums
//==================================================================================================

///
/// # Description
///
/// An enumeration of kernel call numbers.
///
#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum KcallNumber {
    /// Debug.
    Debug = 0,
    /// Get process identifier.
    GetPid,
    /// Get thread identifier.
    GetTid,
    /// Get user identifier.
    GetUid,
    /// Get group identifier.
    GetGid,
    /// Get effective user identifier.
    GetEuid,
    /// Get effective group identifier.
    GetEgid,
    /// Set user identifier.
    SetUid,
    /// Set group identifier.
    SetGid,
    /// Set effective user identifier.
    SetEuid,
    /// Set effective group identifier.
    SetEgid,
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
    /// Invalid.
    Invalid,
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl From<usize> for KcallNumber {
    fn from(number: usize) -> KcallNumber {
        match number {
            0 => KcallNumber::Debug,
            1 => KcallNumber::GetPid,
            2 => KcallNumber::GetTid,
            3 => KcallNumber::GetUid,
            4 => KcallNumber::GetGid,
            5 => KcallNumber::GetEuid,
            6 => KcallNumber::GetEgid,
            7 => KcallNumber::SetUid,
            8 => KcallNumber::SetGid,
            9 => KcallNumber::SetEuid,
            10 => KcallNumber::SetEgid,
            11 => KcallNumber::Exit,
            12 => KcallNumber::CapCtl,
            13 => KcallNumber::Resume,
            14 => KcallNumber::Terminate,
            15 => KcallNumber::EventCtrl,
            16 => KcallNumber::Send,
            17 => KcallNumber::Recv,
            18 => KcallNumber::MemoryMap,
            19 => KcallNumber::MemoryUnmap,
            20 => KcallNumber::MemoryCtrl,
            21 => KcallNumber::MemoryCopy,
            22 => KcallNumber::AllocMmio,
            23 => KcallNumber::FreeMmio,
            24 => KcallNumber::AllocPmio,
            25 => KcallNumber::FreePmio,
            26 => KcallNumber::ReadPmio,
            27 => KcallNumber::WritePmio,
            _ => KcallNumber::Invalid,
        }
    }
}

impl From<u32> for KcallNumber {
    fn from(number: u32) -> KcallNumber {
        From::<usize>::from(number as usize)
    }
}

impl From<KcallNumber> for usize {
    fn from(number: KcallNumber) -> usize {
        number as usize
    }
}

impl From<KcallNumber> for u32 {
    fn from(number: KcallNumber) -> u32 {
        number as u32
    }
}
