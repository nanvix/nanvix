// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_long,
    unistd::sysconf_names,
};

//==================================================================================================
// SysConfigName
//==================================================================================================

///
/// # Description
///
/// System configuration name.
///
#[repr(i32)]
pub enum SysConfigName {
    /// Maximum length of arguments for `exec` functions. (SC_ARG_MAX)
    ArgMax = sysconf_names::SC_ARG_MAX,
    /// Maximum simultaneous processes per real user ID. (SC_CHILD_MAX)
    ChildMax = sysconf_names::SC_CHILD_MAX,
    /// Clock ticks per second. (SC_CLK_TCK)
    ClkTck = sysconf_names::SC_CLK_TCK,
    /// Maximum number of supplementary groups. (SC_NGROUPS_MAX)
    NgroupsMax = sysconf_names::SC_NGROUPS_MAX,
    /// Maximum number of open files. (SC_OPEN_MAX)
    OpenMax = sysconf_names::SC_OPEN_MAX,
    /// Job control supported. (SC_JOB_CONTROL)
    JobControl = sysconf_names::SC_JOB_CONTROL,
    /// Saved set-user-ID and set-group-ID supported. (SC_SAVED_IDS)
    SavedIds = sysconf_names::SC_SAVED_IDS,
    /// POSIX version. (SC_VERSION)
    Version = sysconf_names::SC_VERSION,
    /// Memory page size in bytes. (SC_PAGESIZE)
    PageSize = sysconf_names::SC_PAGESIZE,
    /// Number of configured processors. (SC_NPROCESSORS_CONF)
    NumProcessorsAvailable = sysconf_names::SC_NPROCESSORS_CONF,
    /// Number of online processors. (SC_NPROCESSORS_ONLN)
    NumProcessorsOnline = sysconf_names::SC_NPROCESSORS_ONLN,
    /// Total number of physical pages. (SC_PHYS_PAGES)
    PhysPages = sysconf_names::SC_PHYS_PAGES,
    /// Available physical pages. (SC_AVPHYS_PAGES)
    AvphysPages = sysconf_names::SC_AVPHYS_PAGES,
    /// Maximum number of open message queues per process. (SC_MQ_OPEN_MAX)
    MqOpenMax = sysconf_names::SC_MQ_OPEN_MAX,
    /// Maximum priority for message queues. (SC_MQ_PRIO_MAX)
    MqPrioMax = sysconf_names::SC_MQ_PRIO_MAX,
    /// Maximum number of realtime signals. (SC_RTSIG_MAX)
    RtSigMax = sysconf_names::SC_RTSIG_MAX,
    /// Maximum number of semaphores per process. (SC_SEM_NSEMS_MAX)
    SemNSemsMax = sysconf_names::SC_SEM_NSEMS_MAX,
    /// Maximum value a semaphore may have. (SC_SEM_VALUE_MAX)
    SemValueMax = sysconf_names::SC_SEM_VALUE_MAX,
    /// Maximum number of queued signals. (SC_SIGQUEUE_MAX)
    SigqueueMax = sysconf_names::SC_SIGQUEUE_MAX,
    /// Maximum number of timers per process. (SC_TIMER_MAX)
    TimerMax = sysconf_names::SC_TIMER_MAX,
    /// Maximum number of timezone names. (SC_TZNAME_MAX)
    TznameMax = sysconf_names::SC_TZNAME_MAX,
    /// Asynchronous I/O supported. (SC_ASYNCHRONOUS_IO)
    AsynchronousIo = sysconf_names::SC_ASYNCHRONOUS_IO,
    /// `fsync()` supported. (SC_FSYNC)
    Fsync = sysconf_names::SC_FSYNC,
    /// Memory mapped files supported. (SC_MAPPED_FILES)
    MappedFiles = sysconf_names::SC_MAPPED_FILES,
    /// Memory locking supported. (SC_MEMLOCK)
    Memlock = sysconf_names::SC_MEMLOCK,
    /// Range memory locking supported. (SC_MEMLOCK_RANGE)
    MemlockRange = sysconf_names::SC_MEMLOCK_RANGE,
    /// Memory protection supported. (SC_MEMORY_PROTECTION)
    MemoryProtection = sysconf_names::SC_MEMORY_PROTECTION,
    /// Message passing supported. (SC_MESSAGE_PASSING)
    MessagePassing = sysconf_names::SC_MESSAGE_PASSING,
    /// Prioritized I/O supported. (SC_PRIORITIZED_IO)
    PrioritizedIo = sysconf_names::SC_PRIORITIZED_IO,
    /// Realtime signals supported. (SC_REALTIME_SIGNALS)
    RealtimeSignals = sysconf_names::SC_REALTIME_SIGNALS,
    /// POSIX semaphores supported. (SC_SEMAPHORES)
    Semaphores = sysconf_names::SC_SEMAPHORES,
    /// Shared memory objects supported. (SC_SHARED_MEMORY_OBJECTS)
    SharedMemoryObjects = sysconf_names::SC_SHARED_MEMORY_OBJECTS,
    /// Synchronized I/O supported. (SC_SYNCHRONIZED_IO)
    SynchronizedIo = sysconf_names::SC_SYNCHRONIZED_IO,
    /// POSIX timers supported. (SC_TIMERS)
    Timers = sysconf_names::SC_TIMERS,
    /// Maximum simultaneous AIO list I/O operations. (SC_AIO_LISTIO_MAX)
    AioListioMax = sysconf_names::SC_AIO_LISTIO_MAX,
    /// Maximum queued AIO operations. (SC_AIO_MAX)
    AioMax = sysconf_names::SC_AIO_MAX,
    /// Maximum AIO priority delta. (SC_AIO_PRIO_DELTA_MAX)
    AioPrioDeltaMax = sysconf_names::SC_AIO_PRIO_DELTA_MAX,
    /// Maximum number of timer expiration overruns. (SC_DELAYTIMER_MAX)
    DelayTimerMax = sysconf_names::SC_DELAYTIMER_MAX,
    /// Maximum number of thread-specific data keys. (SC_THREAD_KEYS_MAX)
    ThreadKeysMax = sysconf_names::SC_THREAD_KEYS_MAX,
    /// Minimum thread stack size. (SC_THREAD_STACK_MIN)
    ThreadStackMin = sysconf_names::SC_THREAD_STACK_MIN,
    /// Maximum number of threads per process. (SC_THREAD_THREADS_MAX)
    ThreadThreadsMax = sysconf_names::SC_THREAD_THREADS_MAX,
    /// Maximum length of a terminal device name. (SC_TTY_NAME_MAX)
    TtyNameMax = sysconf_names::SC_TTY_NAME_MAX,
    /// Threads supported. (SC_THREADS)
    Threads = sysconf_names::SC_THREADS,
    /// Thread attribute stack address supported. (SC_THREAD_ATTR_STACKADDR)
    ThreadAttrStackAddr = sysconf_names::SC_THREAD_ATTR_STACKADDR,
    /// Thread attribute stack size supported. (SC_THREAD_ATTR_STACKSIZE)
    ThreadAttrStackSize = sysconf_names::SC_THREAD_ATTR_STACKSIZE,
    /// Thread priority scheduling supported. (SC_THREAD_PRIORITYSCHEDULING)
    ThreadPriorityScheduling = sysconf_names::SC_THREAD_PRIORITYSCHEDULING,
    /// Thread priority inheritance supported. (SC_THREAD_PRIO_INHERIT)
    ThreadPrioInherit = sysconf_names::SC_THREAD_PRIO_INHERIT,
    /// Thread priority protection supported. (SC_THREAD_PRIO_PROTECT)
    ThreadPrioProtect = sysconf_names::SC_THREAD_PRIO_PROTECT,
    /// Process-shared thread synchronization supported. (SC_THREAD_PROCESS_SHARED)
    ThreadProcessShared = sysconf_names::SC_THREAD_PROCESS_SHARED,
    /// Thread-safe functions supported. (SC_THREAD_SAFE_FUNCTIONS)
    ThreadSafeFunctions = sysconf_names::SC_THREAD_SAFE_FUNCTIONS,
    /// Maximum size for `getgr_r` buffer. (SC_GETGR_R_SIZE_MAX)
    GetGrRSizeMax = sysconf_names::SC_GETGR_R_SIZE_MAX,
    /// Maximum size for `getpw_r` buffer. (SC_GETPW_R_SIZE_MAX)
    GetPwRSizeMax = sysconf_names::SC_GETPW_R_SIZE_MAX,
    /// Maximum login name length. (SC_LOGIN_NAME_MAX)
    LoginNameMax = sysconf_names::SC_LOGIN_NAME_MAX,
    /// Maximum thread-specific data destructor iterations. (SC_THREAD_DESTRUCTOR_ITERATIONS)
    ThreadDestructorIterations = sysconf_names::SC_THREAD_DESTRUCTOR_ITERATIONS,
    /// Advisory info supported. (SC_ADVISORY_INFO)
    AdvisoryInfo = sysconf_names::SC_ADVISORY_INFO,
    /// Maximum functions registered with `atexit`. (SC_ATEXIT_MAX)
    AtExitMax = sysconf_names::SC_ATEXIT_MAX,
    /// Barriers supported. (SC_BARRIERS)
    Barriers = sysconf_names::SC_BARRIERS,
    /// Maximum obase value for `bc`. (SC_BC_BASE_MAX)
    BcBaseMax = sysconf_names::SC_BC_BASE_MAX,
    /// Maximum dimension for `bc` arrays. (SC_BC_DIM_MAX)
    BcDimMax = sysconf_names::SC_BC_DIM_MAX,
    /// Maximum scale value for `bc`. (SC_BCSCALE_MAX)
    BcScaleMax = sysconf_names::SC_BCSCALE_MAX,
    /// Maximum string length in `bc`. (SC_BC_STRING_MAX)
    BcStringMax = sysconf_names::SC_BC_STRING_MAX,
    /// Clock selection supported. (SC_CLOCK_SELECTION)
    ClockSelection = sysconf_names::SC_CLOCK_SELECTION,
    /// Maximum number of weight entries in locale collation order. (SC_COLL_WEIGHTS_MAX)
    CollWeightsMax = sysconf_names::SC_COLL_WEIGHTS_MAX,
    /// Process CPU-time clocks supported. (SC_CPUTIME)
    CpuTime = sysconf_names::SC_CPUTIME,
    /// Maximum expression nesting in regular expressions. (SC_EXPR_NEST_MAX)
    ExprNestMax = sysconf_names::SC_EXPR_NEST_MAX,
    /// Maximum host name length. (SC_HOST_NAME_MAX)
    HostNameMax = sysconf_names::SC_HOST_NAME_MAX,
    /// Maximum elements in an I/O vector. (SC_IOV_MAX)
    IovMax = sysconf_names::SC_IOV_MAX,
    /// IPv6 supported. (SC_IPV6)
    Ipv6 = sysconf_names::SC_IPV6,
    /// Maximum length of a utility input line. (SC_LINE_MAX)
    LineMax = sysconf_names::SC_LINE_MAX,
    /// Monotonic clock supported. (SC_MONOTONIC_CLOCK)
    MonotonicClock = sysconf_names::SC_MONOTONIC_CLOCK,
    /// Raw sockets supported. (SC_RAW_SOCKETS)
    RawSockets = sysconf_names::SC_RAW_SOCKETS,
    /// Reader-writer locks supported. (SC_READER_WRITER_LOCKS)
    ReaderWriterLocks = sysconf_names::SC_READER_WRITER_LOCKS,
    /// Regular expressions supported. (SC_REGEXP)
    RegExp = sysconf_names::SC_REGEXP,
    /// Maximum number of repeated occurrences of a regular expression. (SC_RE_DUP_MAX)
    ReDupMax = sysconf_names::SC_RE_DUP_MAX,
    /// Shell supported. (SC_SHELL)
    Shell = sysconf_names::SC_SHELL,
    /// Spawn supported. (SC_SPAWN)
    Spawn = sysconf_names::SC_SPAWN,
    /// Spin locks supported. (SC_SPIN_LOCKS)
    SpinLocks = sysconf_names::SC_SPIN_LOCKS,
    /// Sporadic server scheduling supported. (SC_SPORADIC_SERVER)
    SporadicServer = sysconf_names::SC_SPORADIC_SERVER,
    /// Maximum number of replenishments for sporadic servers. (SC_SS_REPL_MAX)
    SsReplMax = sysconf_names::SC_SS_REPL_MAX,
    /// Maximum number of symbolic link loops. (SC_SYMLOOP_MAX)
    SymLoopMax = sysconf_names::SC_SYMLOOP_MAX,
    /// Per-thread CPU-time clocks supported. (SC_THREAD_CPUTIME)
    ThreadCpuTime = sysconf_names::SC_THREAD_CPUTIME,
    /// Per-thread sporadic server scheduling supported. (SC_THREAD_SPORADIC_SERVER)
    ThreadSporadicServer = sysconf_names::SC_THREAD_SPORADIC_SERVER,
    /// Timeouts supported. (SC_TIMEOUTS)
    Timeouts = sysconf_names::SC_TIMEOUTS,
    /// Trace supported. (SC_TRACE)
    Trace = sysconf_names::SC_TRACE,
    /// Trace event filtering supported. (SC_TRACE_EVENT_FILTER)
    TraceEventFilter = sysconf_names::SC_TRACE_EVENT_FILTER,
    /// Maximum trace event name length. (SC_TRACE_EVENT_NAME_MAX)
    TraceEventNameMax = sysconf_names::SC_TRACE_EVENT_NAME_MAX,
    /// Trace inheritance supported. (SC_TRACE_INHERIT)
    TraceInherit = sysconf_names::SC_TRACE_INHERIT,
    /// Trace logging supported. (SC_TRACE_LOG)
    TraceLog = sysconf_names::SC_TRACE_LOG,
    /// Maximum trace name length. (SC_TRACE_NAME_MAX)
    TraceNameMax = sysconf_names::SC_TRACE_NAME_MAX,
    /// Maximum number of trace streams. (SC_TRACE_SYS_MAX)
    TraceSysMax = sysconf_names::SC_TRACE_SYS_MAX,
    /// Maximum user trace events. (SC_TRACE_USER_EVENT_MAX)
    TraceUserEventMax = sysconf_names::SC_TRACE_USER_EVENT_MAX,
    /// Typed memory objects supported. (SC_TYPED_MEMORY_OBJECTS)
    TypedMemoryObjects = sysconf_names::SC_TYPED_MEMORY_OBJECTS,
    /// XBS5 V7 ILP32 OFF32 configuration. (SC_V7_ILP32_OFF32)
    V7Ilp32Off32 = sysconf_names::SC_V7_ILP32_OFF32,
    /// XBS5 V7 ILP32 OFFBIG configuration. (SC_V7_ILP32_OFFBIG)
    V7Ilp32OffBig = sysconf_names::SC_V7_ILP32_OFFBIG,
    /// XBS5 V7 LP64 OFF64 configuration. (SC_V7_LP64_OFF64)
    V7Lp64Off64 = sysconf_names::SC_V7_LP64_OFF64,
    /// XBS5 V7 LPBIG OFFBIG configuration. (SC_V7_LPBIG_OFFBIG)
    V7LpBigOffBig = sysconf_names::SC_V7_LPBIG_OFFBIG,
    /// X/Open cryptography supported. (SC_XOPEN_CRYPT)
    XopenCrypt = sysconf_names::SC_XOPEN_CRYPT,
    /// X/Open enhanced internationalization supported. (SC_XOPEN_ENH_I18N)
    XopenEnhI18n = sysconf_names::SC_XOPEN_ENH_I18N,
    /// X/Open legacy features supported. (SC_XOPEN_LEGACY)
    XopenLegacy = sysconf_names::SC_XOPEN_LEGACY,
    /// X/Open realtime supported. (SC_XOPEN_REALTIME)
    XopenRealtime = sysconf_names::SC_XOPEN_REALTIME,
    /// Maximum number of streams. (SC_STREAM_MAX)
    StreamMax = sysconf_names::SC_STREAM_MAX,
    /// Priority scheduling supported. (SC_PRIORITYSCHEDULING)
    PriorityScheduling = sysconf_names::SC_PRIORITYSCHEDULING,
    /// X/Open realtime threads supported. (SC_XOPEN_REALTIME_THREADS)
    XopenRealtimeThreads = sysconf_names::SC_XOPEN_REALTIME_THREADS,
    /// X/Open shared memory supported. (SC_XOPEN_SHM)
    XopenShm = sysconf_names::SC_XOPEN_SHM,
    /// X/Open streams supported. (SC_XOPEN_STREAMS)
    XopenStreams = sysconf_names::SC_XOPEN_STREAMS,
    /// X/Open UNIX supported. (SC_XOPEN_UNIX)
    XopenUnix = sysconf_names::SC_XOPEN_UNIX,
    /// X/Open version. (SC_XOPEN_VERSION)
    XopenVersion = sysconf_names::SC_XOPEN_VERSION,
    /// POSIX.2 character terminal supported. (SC_2_CHAR_TERM)
    Posix2CharTerm = sysconf_names::SC_2_CHAR_TERM,
    /// POSIX.2 C language binding supported. (SC_2_C_BIND)
    Posix2CBind = sysconf_names::SC_2_C_BIND,
    /// POSIX.2 C development utilities supported. (SC_2_C_DEV)
    Posix2CDev = sysconf_names::SC_2_C_DEV,
    /// POSIX.2 Fortran development utilities supported. (SC_2_FORT_DEV)
    Posix2FortDev = sysconf_names::SC_2_FORT_DEV,
    /// POSIX.2 Fortran runtime supported. (SC_2_FORT_RUN)
    Posix2FortRun = sysconf_names::SC_2_FORT_RUN,
    /// POSIX.2 localedef utilities supported. (SC_2_LOCALEDEF)
    Posix2Localedef = sysconf_names::SC_2_LOCALEDEF,
    /// POSIX.2 batch system supported. (SC_2_PBS)
    Posix2Pbs = sysconf_names::SC_2_PBS,
    /// POSIX.2 batch accounting supported. (SC_2_PBS_ACCOUNTING)
    Posix2PbsAccounting = sysconf_names::SC_2_PBS_ACCOUNTING,
    /// POSIX.2 batch checkpoint restart supported. (SC_2_PBS_CHECKPOINT)
    Posix2PbsCheckpoint = sysconf_names::SC_2_PBS_CHECKPOINT,
    /// POSIX.2 batch locate supported. (SC_2_PBS_LOCATE)
    Posix2PbsLocate = sysconf_names::SC_2_PBS_LOCATE,
    /// POSIX.2 batch message supported. (SC_2_PBS_MESSAGE)
    Posix2PbsMessage = sysconf_names::SC_2_PBS_MESSAGE,
    /// POSIX.2 batch track supported. (SC_2_PBS_TRACK)
    Posix2PbsTrack = sysconf_names::SC_2_PBS_TRACK,
    /// POSIX.2 software development utilities supported. (SC_2_SW_DEV)
    Posix2SwDev = sysconf_names::SC_2_SW_DEV,
    /// POSIX.2 user portability utilities supported. (SC_2_UPE)
    Posix2Upe = sysconf_names::SC_2_UPE,
    /// POSIX.2 version. (SC_2_VERSION)
    Posix2Version = sysconf_names::SC_2_VERSION,
    /// Thread robust priority inheritance supported. (SC_THREAD_ROBUST_PRIO_INHERIT)
    ThreadRobustPrioInherit = sysconf_names::SC_THREAD_ROBUST_PRIO_INHERIT,
    /// Thread robust priority protection supported. (SC_THREAD_ROBUST_PRIO_PROTECT)
    ThreadRobustPrioProtect = sysconf_names::SC_THREAD_ROBUST_PRIO_PROTECT,
    /// X/Open UUCP supported. (SC_XOPEN_UUCP)
    XopenUucp = sysconf_names::SC_XOPEN_UUCP,
    /// Level 1 instruction cache size. (SC_LEVEL1_ICACHE_SIZE)
    Level1ICacheSize = sysconf_names::SC_LEVEL1_ICACHE_SIZE,
    /// Level 1 instruction cache associativity. (SC_LEVEL1_ICACHE_ASSOC)
    Level1ICacheAssoc = sysconf_names::SC_LEVEL1_ICACHE_ASSOC,
    /// Level 1 instruction cache line size. (SC_LEVEL1_ICACHE_LINESIZE)
    Level1ICacheLineSize = sysconf_names::SC_LEVEL1_ICACHE_LINESIZE,
    /// Level 1 data cache size. (SC_LEVEL1_DCACHE_SIZE)
    Level1DCacheSize = sysconf_names::SC_LEVEL1_DCACHE_SIZE,
    /// Level 1 data cache associativity. (SC_LEVEL1_DCACHE_ASSOC)
    Level1DCacheAssoc = sysconf_names::SC_LEVEL1_DCACHE_ASSOC,
    /// Level 1 data cache line size. (SC_LEVEL1_DCACHE_LINESIZE)
    Level1DCacheLineSize = sysconf_names::SC_LEVEL1_DCACHE_LINESIZE,
    /// Level 2 cache size. (SC_LEVEL2_CACHE_SIZE)
    Level2CacheSize = sysconf_names::SC_LEVEL2_CACHE_SIZE,
    /// Level 2 cache associativity. (SC_LEVEL2_CACHE_ASSOC)
    Level2CacheAssoc = sysconf_names::SC_LEVEL2_CACHE_ASSOC,
    /// Level 2 cache line size. (SC_LEVEL2_CACHE_LINESIZE)
    Level2CacheLineSize = sysconf_names::SC_LEVEL2_CACHE_LINESIZE,
    /// Level 3 cache size. (SC_LEVEL3_CACHE_SIZE)
    Level3CacheSize = sysconf_names::SC_LEVEL3_CACHE_SIZE,
    /// Level 3 cache associativity. (SC_LEVEL3_CACHE_ASSOC)
    Level3CacheAssoc = sysconf_names::SC_LEVEL3_CACHE_ASSOC,
    /// Level 3 cache line size. (SC_LEVEL3_CACHE_LINESIZE)
    Level3CacheLineSize = sysconf_names::SC_LEVEL3_CACHE_LINESIZE,
    /// Level 4 cache size. (SC_LEVEL4_CACHE_SIZE)
    Level4CacheSize = sysconf_names::SC_LEVEL4_CACHE_SIZE,
    /// Level 4 cache associativity. (SC_LEVEL4_CACHE_ASSOC)
    Level4CacheAssoc = sysconf_names::SC_LEVEL4_CACHE_ASSOC,
    /// Level 4 cache line size. (SC_LEVEL4_CACHE_LINESIZE)
    Level4CacheLineSize = sysconf_names::SC_LEVEL4_CACHE_LINESIZE,
    /// POSIX.26 version (non-POSIX extension). (SC_POSIX_26_VERSION)
    Posix26Version = sysconf_names::SC_POSIX_26_VERSION,
}

impl From<SysConfigName> for i32 {
    fn from(name: SysConfigName) -> Self {
        name as i32
    }
}

impl TryFrom<i32> for SysConfigName {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            x if x == sysconf_names::SC_ARG_MAX => Ok(Self::ArgMax),
            x if x == sysconf_names::SC_CHILD_MAX => Ok(Self::ChildMax),
            x if x == sysconf_names::SC_CLK_TCK => Ok(Self::ClkTck),
            x if x == sysconf_names::SC_NGROUPS_MAX => Ok(Self::NgroupsMax),
            x if x == sysconf_names::SC_OPEN_MAX => Ok(Self::OpenMax),
            x if x == sysconf_names::SC_JOB_CONTROL => Ok(Self::JobControl),
            x if x == sysconf_names::SC_SAVED_IDS => Ok(Self::SavedIds),
            x if x == sysconf_names::SC_VERSION => Ok(Self::Version),
            x if x == sysconf_names::SC_PAGESIZE => Ok(Self::PageSize),
            x if x == sysconf_names::SC_NPROCESSORS_CONF => Ok(Self::NumProcessorsAvailable),
            x if x == sysconf_names::SC_NPROCESSORS_ONLN => Ok(Self::NumProcessorsOnline),
            x if x == sysconf_names::SC_PHYS_PAGES => Ok(Self::PhysPages),
            x if x == sysconf_names::SC_AVPHYS_PAGES => Ok(Self::AvphysPages),
            x if x == sysconf_names::SC_MQ_OPEN_MAX => Ok(Self::MqOpenMax),
            x if x == sysconf_names::SC_MQ_PRIO_MAX => Ok(Self::MqPrioMax),
            x if x == sysconf_names::SC_RTSIG_MAX => Ok(Self::RtSigMax),
            x if x == sysconf_names::SC_SEM_NSEMS_MAX => Ok(Self::SemNSemsMax),
            x if x == sysconf_names::SC_SEM_VALUE_MAX => Ok(Self::SemValueMax),
            x if x == sysconf_names::SC_SIGQUEUE_MAX => Ok(Self::SigqueueMax),
            x if x == sysconf_names::SC_TIMER_MAX => Ok(Self::TimerMax),
            x if x == sysconf_names::SC_TZNAME_MAX => Ok(Self::TznameMax),
            x if x == sysconf_names::SC_ASYNCHRONOUS_IO => Ok(Self::AsynchronousIo),
            x if x == sysconf_names::SC_FSYNC => Ok(Self::Fsync),
            x if x == sysconf_names::SC_MAPPED_FILES => Ok(Self::MappedFiles),
            x if x == sysconf_names::SC_MEMLOCK => Ok(Self::Memlock),
            x if x == sysconf_names::SC_MEMLOCK_RANGE => Ok(Self::MemlockRange),
            x if x == sysconf_names::SC_MEMORY_PROTECTION => Ok(Self::MemoryProtection),
            x if x == sysconf_names::SC_MESSAGE_PASSING => Ok(Self::MessagePassing),
            x if x == sysconf_names::SC_PRIORITIZED_IO => Ok(Self::PrioritizedIo),
            x if x == sysconf_names::SC_REALTIME_SIGNALS => Ok(Self::RealtimeSignals),
            x if x == sysconf_names::SC_SEMAPHORES => Ok(Self::Semaphores),
            x if x == sysconf_names::SC_SHARED_MEMORY_OBJECTS => Ok(Self::SharedMemoryObjects),
            x if x == sysconf_names::SC_SYNCHRONIZED_IO => Ok(Self::SynchronizedIo),
            x if x == sysconf_names::SC_TIMERS => Ok(Self::Timers),
            x if x == sysconf_names::SC_AIO_LISTIO_MAX => Ok(Self::AioListioMax),
            x if x == sysconf_names::SC_AIO_MAX => Ok(Self::AioMax),
            x if x == sysconf_names::SC_AIO_PRIO_DELTA_MAX => Ok(Self::AioPrioDeltaMax),
            x if x == sysconf_names::SC_DELAYTIMER_MAX => Ok(Self::DelayTimerMax),
            x if x == sysconf_names::SC_THREAD_KEYS_MAX => Ok(Self::ThreadKeysMax),
            x if x == sysconf_names::SC_THREAD_STACK_MIN => Ok(Self::ThreadStackMin),
            x if x == sysconf_names::SC_THREAD_THREADS_MAX => Ok(Self::ThreadThreadsMax),
            x if x == sysconf_names::SC_TTY_NAME_MAX => Ok(Self::TtyNameMax),
            x if x == sysconf_names::SC_THREADS => Ok(Self::Threads),
            x if x == sysconf_names::SC_THREAD_ATTR_STACKADDR => Ok(Self::ThreadAttrStackAddr),
            x if x == sysconf_names::SC_THREAD_ATTR_STACKSIZE => Ok(Self::ThreadAttrStackSize),
            x if x == sysconf_names::SC_THREAD_PRIORITYSCHEDULING => {
                Ok(Self::ThreadPriorityScheduling)
            },
            x if x == sysconf_names::SC_THREAD_PRIO_INHERIT => Ok(Self::ThreadPrioInherit),
            x if x == sysconf_names::SC_THREAD_PRIO_PROTECT => Ok(Self::ThreadPrioProtect),
            x if x == sysconf_names::SC_THREAD_PROCESS_SHARED => Ok(Self::ThreadProcessShared),
            x if x == sysconf_names::SC_THREAD_SAFE_FUNCTIONS => Ok(Self::ThreadSafeFunctions),
            x if x == sysconf_names::SC_GETGR_R_SIZE_MAX => Ok(Self::GetGrRSizeMax),
            x if x == sysconf_names::SC_GETPW_R_SIZE_MAX => Ok(Self::GetPwRSizeMax),
            x if x == sysconf_names::SC_LOGIN_NAME_MAX => Ok(Self::LoginNameMax),
            x if x == sysconf_names::SC_THREAD_DESTRUCTOR_ITERATIONS => {
                Ok(Self::ThreadDestructorIterations)
            },
            x if x == sysconf_names::SC_ADVISORY_INFO => Ok(Self::AdvisoryInfo),
            x if x == sysconf_names::SC_ATEXIT_MAX => Ok(Self::AtExitMax),
            x if x == sysconf_names::SC_BARRIERS => Ok(Self::Barriers),
            x if x == sysconf_names::SC_BC_BASE_MAX => Ok(Self::BcBaseMax),
            x if x == sysconf_names::SC_BC_DIM_MAX => Ok(Self::BcDimMax),
            x if x == sysconf_names::SC_BCSCALE_MAX => Ok(Self::BcScaleMax),
            x if x == sysconf_names::SC_BC_STRING_MAX => Ok(Self::BcStringMax),
            x if x == sysconf_names::SC_CLOCK_SELECTION => Ok(Self::ClockSelection),
            x if x == sysconf_names::SC_COLL_WEIGHTS_MAX => Ok(Self::CollWeightsMax),
            x if x == sysconf_names::SC_CPUTIME => Ok(Self::CpuTime),
            x if x == sysconf_names::SC_EXPR_NEST_MAX => Ok(Self::ExprNestMax),
            x if x == sysconf_names::SC_HOST_NAME_MAX => Ok(Self::HostNameMax),
            x if x == sysconf_names::SC_IOV_MAX => Ok(Self::IovMax),
            x if x == sysconf_names::SC_IPV6 => Ok(Self::Ipv6),
            x if x == sysconf_names::SC_LINE_MAX => Ok(Self::LineMax),
            x if x == sysconf_names::SC_MONOTONIC_CLOCK => Ok(Self::MonotonicClock),
            x if x == sysconf_names::SC_RAW_SOCKETS => Ok(Self::RawSockets),
            x if x == sysconf_names::SC_READER_WRITER_LOCKS => Ok(Self::ReaderWriterLocks),
            x if x == sysconf_names::SC_REGEXP => Ok(Self::RegExp),
            x if x == sysconf_names::SC_RE_DUP_MAX => Ok(Self::ReDupMax),
            x if x == sysconf_names::SC_SHELL => Ok(Self::Shell),
            x if x == sysconf_names::SC_SPAWN => Ok(Self::Spawn),
            x if x == sysconf_names::SC_SPIN_LOCKS => Ok(Self::SpinLocks),
            x if x == sysconf_names::SC_SPORADIC_SERVER => Ok(Self::SporadicServer),
            x if x == sysconf_names::SC_SS_REPL_MAX => Ok(Self::SsReplMax),
            x if x == sysconf_names::SC_SYMLOOP_MAX => Ok(Self::SymLoopMax),
            x if x == sysconf_names::SC_THREAD_CPUTIME => Ok(Self::ThreadCpuTime),
            x if x == sysconf_names::SC_THREAD_SPORADIC_SERVER => Ok(Self::ThreadSporadicServer),
            x if x == sysconf_names::SC_TIMEOUTS => Ok(Self::Timeouts),
            x if x == sysconf_names::SC_TRACE => Ok(Self::Trace),
            x if x == sysconf_names::SC_TRACE_EVENT_FILTER => Ok(Self::TraceEventFilter),
            x if x == sysconf_names::SC_TRACE_EVENT_NAME_MAX => Ok(Self::TraceEventNameMax),
            x if x == sysconf_names::SC_TRACE_INHERIT => Ok(Self::TraceInherit),
            x if x == sysconf_names::SC_TRACE_LOG => Ok(Self::TraceLog),
            x if x == sysconf_names::SC_TRACE_NAME_MAX => Ok(Self::TraceNameMax),
            x if x == sysconf_names::SC_TRACE_SYS_MAX => Ok(Self::TraceSysMax),
            x if x == sysconf_names::SC_TRACE_USER_EVENT_MAX => Ok(Self::TraceUserEventMax),
            x if x == sysconf_names::SC_TYPED_MEMORY_OBJECTS => Ok(Self::TypedMemoryObjects),
            x if x == sysconf_names::SC_V7_ILP32_OFF32 => Ok(Self::V7Ilp32Off32),
            x if x == sysconf_names::SC_V7_ILP32_OFFBIG => Ok(Self::V7Ilp32OffBig),
            x if x == sysconf_names::SC_V7_LP64_OFF64 => Ok(Self::V7Lp64Off64),
            x if x == sysconf_names::SC_V7_LPBIG_OFFBIG => Ok(Self::V7LpBigOffBig),
            x if x == sysconf_names::SC_XOPEN_CRYPT => Ok(Self::XopenCrypt),
            x if x == sysconf_names::SC_XOPEN_ENH_I18N => Ok(Self::XopenEnhI18n),
            x if x == sysconf_names::SC_XOPEN_LEGACY => Ok(Self::XopenLegacy),
            x if x == sysconf_names::SC_XOPEN_REALTIME => Ok(Self::XopenRealtime),
            x if x == sysconf_names::SC_STREAM_MAX => Ok(Self::StreamMax),
            x if x == sysconf_names::SC_PRIORITYSCHEDULING => Ok(Self::PriorityScheduling),
            x if x == sysconf_names::SC_XOPEN_REALTIME_THREADS => Ok(Self::XopenRealtimeThreads),
            x if x == sysconf_names::SC_XOPEN_SHM => Ok(Self::XopenShm),
            x if x == sysconf_names::SC_XOPEN_STREAMS => Ok(Self::XopenStreams),
            x if x == sysconf_names::SC_XOPEN_UNIX => Ok(Self::XopenUnix),
            x if x == sysconf_names::SC_XOPEN_VERSION => Ok(Self::XopenVersion),
            x if x == sysconf_names::SC_2_CHAR_TERM => Ok(Self::Posix2CharTerm),
            x if x == sysconf_names::SC_2_C_BIND => Ok(Self::Posix2CBind),
            x if x == sysconf_names::SC_2_C_DEV => Ok(Self::Posix2CDev),
            x if x == sysconf_names::SC_2_FORT_DEV => Ok(Self::Posix2FortDev),
            x if x == sysconf_names::SC_2_FORT_RUN => Ok(Self::Posix2FortRun),
            x if x == sysconf_names::SC_2_LOCALEDEF => Ok(Self::Posix2Localedef),
            x if x == sysconf_names::SC_2_PBS => Ok(Self::Posix2Pbs),
            x if x == sysconf_names::SC_2_PBS_ACCOUNTING => Ok(Self::Posix2PbsAccounting),
            x if x == sysconf_names::SC_2_PBS_CHECKPOINT => Ok(Self::Posix2PbsCheckpoint),
            x if x == sysconf_names::SC_2_PBS_LOCATE => Ok(Self::Posix2PbsLocate),
            x if x == sysconf_names::SC_2_PBS_MESSAGE => Ok(Self::Posix2PbsMessage),
            x if x == sysconf_names::SC_2_PBS_TRACK => Ok(Self::Posix2PbsTrack),
            x if x == sysconf_names::SC_2_SW_DEV => Ok(Self::Posix2SwDev),
            x if x == sysconf_names::SC_2_UPE => Ok(Self::Posix2Upe),
            x if x == sysconf_names::SC_2_VERSION => Ok(Self::Posix2Version),
            x if x == sysconf_names::SC_THREAD_ROBUST_PRIO_INHERIT => {
                Ok(Self::ThreadRobustPrioInherit)
            },
            x if x == sysconf_names::SC_THREAD_ROBUST_PRIO_PROTECT => {
                Ok(Self::ThreadRobustPrioProtect)
            },
            x if x == sysconf_names::SC_XOPEN_UUCP => Ok(Self::XopenUucp),
            x if x == sysconf_names::SC_LEVEL1_ICACHE_SIZE => Ok(Self::Level1ICacheSize),
            x if x == sysconf_names::SC_LEVEL1_ICACHE_ASSOC => Ok(Self::Level1ICacheAssoc),
            x if x == sysconf_names::SC_LEVEL1_ICACHE_LINESIZE => Ok(Self::Level1ICacheLineSize),
            x if x == sysconf_names::SC_LEVEL1_DCACHE_SIZE => Ok(Self::Level1DCacheSize),
            x if x == sysconf_names::SC_LEVEL1_DCACHE_ASSOC => Ok(Self::Level1DCacheAssoc),
            x if x == sysconf_names::SC_LEVEL1_DCACHE_LINESIZE => Ok(Self::Level1DCacheLineSize),
            x if x == sysconf_names::SC_LEVEL2_CACHE_SIZE => Ok(Self::Level2CacheSize),
            x if x == sysconf_names::SC_LEVEL2_CACHE_ASSOC => Ok(Self::Level2CacheAssoc),
            x if x == sysconf_names::SC_LEVEL2_CACHE_LINESIZE => Ok(Self::Level2CacheLineSize),
            x if x == sysconf_names::SC_LEVEL3_CACHE_SIZE => Ok(Self::Level3CacheSize),
            x if x == sysconf_names::SC_LEVEL3_CACHE_ASSOC => Ok(Self::Level3CacheAssoc),
            x if x == sysconf_names::SC_LEVEL3_CACHE_LINESIZE => Ok(Self::Level3CacheLineSize),
            x if x == sysconf_names::SC_LEVEL4_CACHE_SIZE => Ok(Self::Level4CacheSize),
            x if x == sysconf_names::SC_LEVEL4_CACHE_ASSOC => Ok(Self::Level4CacheAssoc),
            x if x == sysconf_names::SC_LEVEL4_CACHE_LINESIZE => Ok(Self::Level4CacheLineSize),
            x if x == sysconf_names::SC_POSIX_26_VERSION => Ok(Self::Posix26Version),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid system configuration name")),
        }
    }
}

//==================================================================================================
// SysConfigValue
//==================================================================================================

///
/// # Description
///
/// System configuration value.
///
pub struct SysConfigValue {
    /// Raw value.
    value: c_long,
}

impl From<i8> for SysConfigValue {
    fn from(value: i8) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<i16> for SysConfigValue {
    fn from(value: i16) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<i32> for SysConfigValue {
    fn from(value: i32) -> Self {
        Self { value }
    }
}

impl TryFrom<isize> for SysConfigValue {
    type Error = Error;

    fn try_from(value: isize) -> Result<Self, Self::Error> {
        Ok(Self {
            value: value
                .try_into()
                .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "value out of range"))?,
        })
    }
}

impl From<u8> for SysConfigValue {
    fn from(value: u8) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl From<u16> for SysConfigValue {
    fn from(value: u16) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl TryFrom<u32> for SysConfigValue {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self {
            value: value
                .try_into()
                .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "value out of range"))?,
        })
    }
}

impl TryFrom<usize> for SysConfigValue {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(Self {
            value: value
                .try_into()
                .map_err(|_| Error::new(ErrorCode::ValueOutOfRange, "value out of range"))?,
        })
    }
}

impl From<SysConfigValue> for c_long {
    fn from(value: SysConfigValue) -> Self {
        value.value
    }
}
