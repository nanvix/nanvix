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
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum KcallNumber {
    /// Debug.
    Debug = KcallNumber::NR_DEBUG_SYSCALL,
    /// Get process identifier.
    GetPid = KcallNumber::NR_GET_PID_SYSCALL,
    /// Get thread identifier.
    GetTid = KcallNumber::NR_GET_TID_SYSCALL,
    /// Terminate the calling process.
    Exit = KcallNumber::NR_EXIT_SYSCALL,
    /// Control capabilities.
    CapCtl = KcallNumber::NR_CAP_CTL_SYSCALL,
    /// Resumes an interrupted process.
    Resume = KcallNumber::NR_RESUME_SYSCALL,
    /// Terminates a process.
    Terminate = KcallNumber::NR_TERMINATE_SYSCALL,
    /// Controls events.
    EventCtrl = KcallNumber::NR_EVENT_CTRL_SYSCALL,
    /// Sends a message.
    Send = KcallNumber::NR_SEND_SYSCALL,
    /// Receives a message.
    Recv = KcallNumber::NR_RECV_SYSCALL,
    /// Map memory page.
    MemoryMap = KcallNumber::NR_MEMORY_MAP_SYSCALL,
    /// Unmap memory page.
    MemoryUnmap = KcallNumber::NR_MEMORY_UNMAP_SYSCALL,
    /// Controls a memory page.
    MemoryCtrl = KcallNumber::NR_MEMORY_CTRL_SYSCALL,
    /// Copies a memory page.
    MemoryCopy = KcallNumber::NR_MEMORY_COPY_SYSCALL,
    /// Allocates a memory-mapped I/O region.
    AllocMmio = KcallNumber::NR_ALLOC_MMIO_SYSCALL,
    /// Releases a memory-mapped I/O region.
    FreeMmio = KcallNumber::NR_FREE_MMIO_SYSCALL,
    /// Retrieves metadata for a memory-mapped I/O region.
    MmioInfo = KcallNumber::NR_MMIO_INFO_SYSCALL,
    /// Allocates a port-mapped I/O port.
    AllocPmio = KcallNumber::NR_ALLOC_PMIO_SYSCALL,
    /// Frees a port-mapped I/O port.
    FreePmio = KcallNumber::NR_FREE_PMIO_SYSCALL,
    /// Reads a value from a port-mapped I/O port.
    ReadPmio = KcallNumber::NR_READ_PMIO_SYSCALL,
    /// Writes a value to a port-mapped I/O port.
    WritePmio = KcallNumber::NR_WRITE_PMIO_SYSCALL,
    /// Yields the processor.
    SchedulerYield = KcallNumber::NR_SCHEDULER_YIELD_SYSCALL,
    /// Create a new thread.
    CreateThread = KcallNumber::NR_CREATE_THREAD_SYSCALL,
    /// Terminates the calling thread.
    ExitThread = KcallNumber::NR_EXIT_THREAD_SYSCALL,
    /// Joins with a terminated thread.
    JoinThread = KcallNumber::NR_JOIN_THREAD_SYSCALL,
    /// Locks a mutex.
    MutexLock = KcallNumber::NR_MUTEX_LOCK_SYSCALL,
    /// Unlocks a mutex.
    MutexUnlock = KcallNumber::NR_MUTEX_UNLOCK_SYSCALL,
    /// Signals a condition variable.
    CondSignal = KcallNumber::NR_COND_SIGNAL_SYSCALL,
    /// Waits on a condition variable.
    CondWait = KcallNumber::NR_COND_WAIT_SYSCALL,
    /// Gets the current system time.
    GetTime = KcallNumber::NR_GET_TIME_SYSCALL,
    /// Puts the calling thread to sleep.
    Sleep = KcallNumber::NR_SLEEP_SYSCALL,
    /// Sets the thread-local storage.
    SetThreadDataArea = KcallNumber::NR_SET_TDA_SYSCALL,
    /// Gets the thread-local storage.
    GetThreadDataArea = KcallNumber::NR_GET_TDA_SYSCALL,
    /// Initiates a rendezvous send transfer.
    Push = KcallNumber::NR_PUSH_SYSCALL,
    /// Initiates a rendezvous receive transfer.
    Pull = KcallNumber::NR_PULL_SYSCALL,
    /// Creates a snapshot of the virtual machine.
    Snapshot = KcallNumber::NR_SNAPSHOT_SYSCALL,
    /// Invalid kernel call.
    Invalid = KcallNumber::NR_INVALID_SYSCALL,
}

impl KcallNumber {
    const NR_DEBUG_SYSCALL: u32 = 0;
    const NR_GET_PID_SYSCALL: u32 = 1;
    const NR_GET_TID_SYSCALL: u32 = 2;
    const NR_EXIT_SYSCALL: u32 = 3;
    const NR_CAP_CTL_SYSCALL: u32 = 4;
    const NR_RESUME_SYSCALL: u32 = 5;
    const NR_TERMINATE_SYSCALL: u32 = 6;
    const NR_EVENT_CTRL_SYSCALL: u32 = 7;
    const NR_SEND_SYSCALL: u32 = 8;
    const NR_RECV_SYSCALL: u32 = 9;
    const NR_MEMORY_MAP_SYSCALL: u32 = 10;
    const NR_MEMORY_UNMAP_SYSCALL: u32 = 11;
    const NR_MEMORY_CTRL_SYSCALL: u32 = 12;
    const NR_MEMORY_COPY_SYSCALL: u32 = 13;
    const NR_ALLOC_MMIO_SYSCALL: u32 = 14;
    const NR_FREE_MMIO_SYSCALL: u32 = 15;
    const NR_MMIO_INFO_SYSCALL: u32 = 32;
    const NR_ALLOC_PMIO_SYSCALL: u32 = 16;
    const NR_FREE_PMIO_SYSCALL: u32 = 17;
    const NR_READ_PMIO_SYSCALL: u32 = 18;
    const NR_WRITE_PMIO_SYSCALL: u32 = 19;
    const NR_SCHEDULER_YIELD_SYSCALL: u32 = 20;
    const NR_CREATE_THREAD_SYSCALL: u32 = 21;
    const NR_EXIT_THREAD_SYSCALL: u32 = 22;
    const NR_JOIN_THREAD_SYSCALL: u32 = 23;
    const NR_MUTEX_LOCK_SYSCALL: u32 = 24;
    const NR_MUTEX_UNLOCK_SYSCALL: u32 = 25;
    const NR_COND_SIGNAL_SYSCALL: u32 = 26;
    const NR_COND_WAIT_SYSCALL: u32 = 27;
    const NR_GET_TIME_SYSCALL: u32 = 28;
    const NR_SLEEP_SYSCALL: u32 = 29;
    const NR_SET_TDA_SYSCALL: u32 = 30;
    const NR_GET_TDA_SYSCALL: u32 = 31;
    // NOTE: number 32 is already used by NR_MMIO_INFO_SYSCALL (assigned out of order above).
    const NR_PUSH_SYSCALL: u32 = 33;
    const NR_PULL_SYSCALL: u32 = 34;
    const NR_SNAPSHOT_SYSCALL: u32 = 35;
    const NR_INVALID_SYSCALL: u32 = u32::MAX;
}

// Manual conversion from u32 to KcallNumber
impl From<u32> for KcallNumber {
    fn from(value: u32) -> Self {
        match value {
            Self::NR_DEBUG_SYSCALL => KcallNumber::Debug,
            Self::NR_GET_PID_SYSCALL => KcallNumber::GetPid,
            Self::NR_GET_TID_SYSCALL => KcallNumber::GetTid,
            Self::NR_EXIT_SYSCALL => KcallNumber::Exit,
            Self::NR_CAP_CTL_SYSCALL => KcallNumber::CapCtl,
            Self::NR_RESUME_SYSCALL => KcallNumber::Resume,
            Self::NR_TERMINATE_SYSCALL => KcallNumber::Terminate,
            Self::NR_EVENT_CTRL_SYSCALL => KcallNumber::EventCtrl,
            Self::NR_SEND_SYSCALL => KcallNumber::Send,
            Self::NR_RECV_SYSCALL => KcallNumber::Recv,
            Self::NR_MEMORY_MAP_SYSCALL => KcallNumber::MemoryMap,
            Self::NR_MEMORY_UNMAP_SYSCALL => KcallNumber::MemoryUnmap,
            Self::NR_MEMORY_CTRL_SYSCALL => KcallNumber::MemoryCtrl,
            Self::NR_MEMORY_COPY_SYSCALL => KcallNumber::MemoryCopy,
            Self::NR_ALLOC_MMIO_SYSCALL => KcallNumber::AllocMmio,
            Self::NR_FREE_MMIO_SYSCALL => KcallNumber::FreeMmio,
            Self::NR_MMIO_INFO_SYSCALL => KcallNumber::MmioInfo,
            Self::NR_ALLOC_PMIO_SYSCALL => KcallNumber::AllocPmio,
            Self::NR_FREE_PMIO_SYSCALL => KcallNumber::FreePmio,
            Self::NR_READ_PMIO_SYSCALL => KcallNumber::ReadPmio,
            Self::NR_WRITE_PMIO_SYSCALL => KcallNumber::WritePmio,
            Self::NR_SCHEDULER_YIELD_SYSCALL => KcallNumber::SchedulerYield,
            Self::NR_CREATE_THREAD_SYSCALL => KcallNumber::CreateThread,
            Self::NR_EXIT_THREAD_SYSCALL => KcallNumber::ExitThread,
            Self::NR_JOIN_THREAD_SYSCALL => KcallNumber::JoinThread,
            Self::NR_MUTEX_LOCK_SYSCALL => KcallNumber::MutexLock,
            Self::NR_MUTEX_UNLOCK_SYSCALL => KcallNumber::MutexUnlock,
            Self::NR_COND_SIGNAL_SYSCALL => KcallNumber::CondSignal,
            Self::NR_COND_WAIT_SYSCALL => KcallNumber::CondWait,
            Self::NR_GET_TIME_SYSCALL => KcallNumber::GetTime,
            Self::NR_SLEEP_SYSCALL => KcallNumber::Sleep,
            Self::NR_SET_TDA_SYSCALL => KcallNumber::SetThreadDataArea,
            Self::NR_GET_TDA_SYSCALL => KcallNumber::GetThreadDataArea,
            Self::NR_PUSH_SYSCALL => KcallNumber::Push,
            Self::NR_PULL_SYSCALL => KcallNumber::Pull,
            Self::NR_SNAPSHOT_SYSCALL => KcallNumber::Snapshot,
            _ => KcallNumber::Invalid,
        }
    }
}

// Manual conversion from KcallNumber to u32
impl From<KcallNumber> for u32 {
    fn from(k: KcallNumber) -> Self {
        match k {
            KcallNumber::Debug => KcallNumber::NR_DEBUG_SYSCALL,
            KcallNumber::GetPid => KcallNumber::NR_GET_PID_SYSCALL,
            KcallNumber::GetTid => KcallNumber::NR_GET_TID_SYSCALL,
            KcallNumber::Exit => KcallNumber::NR_EXIT_SYSCALL,
            KcallNumber::CapCtl => KcallNumber::NR_CAP_CTL_SYSCALL,
            KcallNumber::Resume => KcallNumber::NR_RESUME_SYSCALL,
            KcallNumber::Terminate => KcallNumber::NR_TERMINATE_SYSCALL,
            KcallNumber::EventCtrl => KcallNumber::NR_EVENT_CTRL_SYSCALL,
            KcallNumber::Send => KcallNumber::NR_SEND_SYSCALL,
            KcallNumber::Recv => KcallNumber::NR_RECV_SYSCALL,
            KcallNumber::MemoryMap => KcallNumber::NR_MEMORY_MAP_SYSCALL,
            KcallNumber::MemoryUnmap => KcallNumber::NR_MEMORY_UNMAP_SYSCALL,
            KcallNumber::MemoryCtrl => KcallNumber::NR_MEMORY_CTRL_SYSCALL,
            KcallNumber::MemoryCopy => KcallNumber::NR_MEMORY_COPY_SYSCALL,
            KcallNumber::AllocMmio => KcallNumber::NR_ALLOC_MMIO_SYSCALL,
            KcallNumber::FreeMmio => KcallNumber::NR_FREE_MMIO_SYSCALL,
            KcallNumber::MmioInfo => KcallNumber::NR_MMIO_INFO_SYSCALL,
            KcallNumber::AllocPmio => KcallNumber::NR_ALLOC_PMIO_SYSCALL,
            KcallNumber::FreePmio => KcallNumber::NR_FREE_PMIO_SYSCALL,
            KcallNumber::ReadPmio => KcallNumber::NR_READ_PMIO_SYSCALL,
            KcallNumber::WritePmio => KcallNumber::NR_WRITE_PMIO_SYSCALL,
            KcallNumber::SchedulerYield => KcallNumber::NR_SCHEDULER_YIELD_SYSCALL,
            KcallNumber::CreateThread => KcallNumber::NR_CREATE_THREAD_SYSCALL,
            KcallNumber::ExitThread => KcallNumber::NR_EXIT_THREAD_SYSCALL,
            KcallNumber::JoinThread => KcallNumber::NR_JOIN_THREAD_SYSCALL,
            KcallNumber::MutexLock => KcallNumber::NR_MUTEX_LOCK_SYSCALL,
            KcallNumber::MutexUnlock => KcallNumber::NR_MUTEX_UNLOCK_SYSCALL,
            KcallNumber::CondSignal => KcallNumber::NR_COND_SIGNAL_SYSCALL,
            KcallNumber::CondWait => KcallNumber::NR_COND_WAIT_SYSCALL,
            KcallNumber::GetTime => KcallNumber::NR_GET_TIME_SYSCALL,
            KcallNumber::Sleep => KcallNumber::NR_SLEEP_SYSCALL,
            KcallNumber::SetThreadDataArea => KcallNumber::NR_SET_TDA_SYSCALL,
            KcallNumber::GetThreadDataArea => KcallNumber::NR_GET_TDA_SYSCALL,
            KcallNumber::Push => KcallNumber::NR_PUSH_SYSCALL,
            KcallNumber::Pull => KcallNumber::NR_PULL_SYSCALL,
            KcallNumber::Snapshot => KcallNumber::NR_SNAPSHOT_SYSCALL,
            KcallNumber::Invalid => KcallNumber::NR_INVALID_SYSCALL,
        }
    }
}
