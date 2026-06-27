// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// IDT vector used for the user→kernel call trap.
///
/// This vector is deliberately **not** `0x80`, because `0x80` collides with Linux's i386 syscall
/// gate. Using `0x80` makes Nanvix user binaries accidentally "executable" on an x86_64 Linux host,
/// where each `kcall*` turns into an arbitrary Linux syscall and the process exits in undefined
/// ways. `0x81` is unused on Linux (and Windows), so a Nanvix binary run on such a host
/// deterministically faults (`#GP` → `SIGSEGV`) instead of behaving unpredictably.
///
/// This constant is shared between the kernel IDT setup and the user-space kcall assembly so the
/// two sides cannot drift apart.
///
pub const KCALL_VECTOR: u8 = 0x81;

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
    /// Get parent process identifier.
    GetPpid = KcallNumber::NR_GET_PPID_SYSCALL,
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
    /// Detaches a thread so it is auto-harvested on exit.
    DetachThread = KcallNumber::NR_DETACH_THREAD_SYSCALL,
    /// Duplicates the calling process.
    Duplicate = KcallNumber::NR_DUPLICATE_SYSCALL,
    /// Replaces the image of the calling process.
    Execv = KcallNumber::NR_EXECV_SYSCALL,
    /// Gets and/or sets the disposition of a signal.
    Sigaction = KcallNumber::NR_SIGACTION_SYSCALL,
    /// Gets and/or modifies the calling thread's blocked signal mask.
    Sigprocmask = KcallNumber::NR_SIGPROCMASK_SYSCALL,
    /// Posts a signal to a target process.
    Kill = KcallNumber::NR_KILL_SYSCALL,
    /// Restores the calling thread's context after a signal handler returns.
    Sigreturn = KcallNumber::NR_SIGRETURN_SYSCALL,
    /// Retrieves the set of pending-but-blocked signals.
    Sigpending = KcallNumber::NR_SIGPENDING_SYSCALL,
    /// Atomically sets the signal mask and blocks until a signal is delivered.
    Sigsuspend = KcallNumber::NR_SIGSUSPEND_SYSCALL,
    /// Registers the calling process's user-space signal-return trampoline (restorer).
    SigRestorer = KcallNumber::NR_SIG_RESTORER_SYSCALL,
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
    const NR_DETACH_THREAD_SYSCALL: u32 = 36;
    const NR_DUPLICATE_SYSCALL: u32 = 37;
    const NR_GET_PPID_SYSCALL: u32 = 38;
    const NR_EXECV_SYSCALL: u32 = 39;
    const NR_SIG_RESTORER_SYSCALL: u32 = 40;
    const NR_SIGACTION_SYSCALL: u32 = 41;
    const NR_SIGPROCMASK_SYSCALL: u32 = 42;
    const NR_KILL_SYSCALL: u32 = 43;
    const NR_SIGRETURN_SYSCALL: u32 = 44;
    const NR_SIGPENDING_SYSCALL: u32 = 45;
    const NR_SIGSUSPEND_SYSCALL: u32 = 46;
    const NR_INVALID_SYSCALL: u32 = u32::MAX;
}

// Manual conversion from u32 to KcallNumber
impl From<u32> for KcallNumber {
    fn from(value: u32) -> Self {
        match value {
            Self::NR_DEBUG_SYSCALL => KcallNumber::Debug,
            Self::NR_GET_PID_SYSCALL => KcallNumber::GetPid,
            Self::NR_GET_PPID_SYSCALL => KcallNumber::GetPpid,
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
            Self::NR_DETACH_THREAD_SYSCALL => KcallNumber::DetachThread,
            Self::NR_DUPLICATE_SYSCALL => KcallNumber::Duplicate,
            Self::NR_EXECV_SYSCALL => KcallNumber::Execv,
            Self::NR_SIG_RESTORER_SYSCALL => KcallNumber::SigRestorer,
            Self::NR_SIGACTION_SYSCALL => KcallNumber::Sigaction,
            Self::NR_SIGPROCMASK_SYSCALL => KcallNumber::Sigprocmask,
            Self::NR_KILL_SYSCALL => KcallNumber::Kill,
            Self::NR_SIGRETURN_SYSCALL => KcallNumber::Sigreturn,
            Self::NR_SIGPENDING_SYSCALL => KcallNumber::Sigpending,
            Self::NR_SIGSUSPEND_SYSCALL => KcallNumber::Sigsuspend,
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
            KcallNumber::GetPpid => KcallNumber::NR_GET_PPID_SYSCALL,
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
            KcallNumber::DetachThread => KcallNumber::NR_DETACH_THREAD_SYSCALL,
            KcallNumber::Duplicate => KcallNumber::NR_DUPLICATE_SYSCALL,
            KcallNumber::Execv => KcallNumber::NR_EXECV_SYSCALL,
            KcallNumber::SigRestorer => KcallNumber::NR_SIG_RESTORER_SYSCALL,
            KcallNumber::Sigaction => KcallNumber::NR_SIGACTION_SYSCALL,
            KcallNumber::Sigprocmask => KcallNumber::NR_SIGPROCMASK_SYSCALL,
            KcallNumber::Kill => KcallNumber::NR_KILL_SYSCALL,
            KcallNumber::Sigreturn => KcallNumber::NR_SIGRETURN_SYSCALL,
            KcallNumber::Sigpending => KcallNumber::NR_SIGPENDING_SYSCALL,
            KcallNumber::Sigsuspend => KcallNumber::NR_SIGSUSPEND_SYSCALL,
            KcallNumber::Invalid => KcallNumber::NR_INVALID_SYSCALL,
        }
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// The kcall trap vector must never be `0x80`, because that vector collides with the Linux
    /// i386 syscall gate and makes Nanvix user binaries behave unpredictably when run on a Linux
    /// host. See the module-level documentation of [`KCALL_VECTOR`] for details.
    #[test]
    fn test_kcall_vector_avoids_linux_syscall_gate() {
        assert_ne!(KCALL_VECTOR, 0x80);
    }

    #[test]
    fn test_signal_kcall_numbers_round_trip() {
        let signal_kcalls: [(KcallNumber, u32); 6] = [
            (KcallNumber::Sigaction, 41),
            (KcallNumber::Sigprocmask, 42),
            (KcallNumber::Kill, 43),
            (KcallNumber::Sigreturn, 44),
            (KcallNumber::Sigpending, 45),
            (KcallNumber::Sigsuspend, 46),
        ];

        for (kcall, number) in signal_kcalls {
            assert_eq!(u32::from(kcall), number);
            assert_eq!(KcallNumber::from(number), kcall);
        }
    }
}
