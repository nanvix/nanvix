// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::size_t,
};

//==================================================================================================
// Constants
//==================================================================================================

pub mod file_seek {
    use crate::ffi::c_int;

    /// Seek relative to start-of-file.
    pub const SEEK_SET: c_int = 0;
    /// Seek relative to current position.
    pub const SEEK_CUR: c_int = 1;
    /// Seek relative to end-of-file.
    pub const SEEK_END: c_int = 2;
    /// Seek forwards from offset relative to start-of-file for a position within a hole.
    pub const SEEK_HOLE: c_int = 3;
    /// Seek forwards from offset relative to start-of-file for a position not within a hole.
    pub const SEEK_DATA: c_int = 4;
}

/// File number of standard input.
pub const STDIN_FILENO: i32 = 0;
/// File number of standard output.
pub const STDOUT_FILENO: i32 = 1;
/// File number of standard error.
pub const STDERR_FILENO: i32 = 2;

/// System configuration names for `sysconf()`.
pub mod sysconf_names {
    use crate::ffi::c_int;

    /// System configuration name for ARG_MAX.
    pub const SC_ARG_MAX: c_int = 0;
    /// System configuration name for CHILD_MAX.
    pub const SC_CHILD_MAX: c_int = 1;
    /// System configuration name for CLK_TCK.
    pub const SC_CLK_TCK: c_int = 2;
    /// System configuration name for NGROUPS_MAX.
    pub const SC_NGROUPS_MAX: c_int = 3;
    /// System configuration name for OPEN_MAX.
    pub const SC_OPEN_MAX: c_int = 4;
    /// System configuration name for job control support.
    pub const SC_JOB_CONTROL: c_int = 5;
    /// System configuration name for saved IDs.
    pub const SC_SAVED_IDS: c_int = 6;
    /// System configuration name for POSIX version.
    pub const SC_VERSION: c_int = 7;
    /// System configuration name for page size.
    pub const SC_PAGESIZE: c_int = 8;
    /// Alias of SC_PAGESIZE.
    pub const SC_PAGE_SIZE: c_int = SC_PAGESIZE;
    /// System configuration name for number of configured processors.
    pub const SC_NPROCESSORS_CONF: c_int = 9;
    /// System configuration name for number of online processors.
    pub const SC_NPROCESSORS_ONLN: c_int = 10;
    /// System configuration name for total number of physical pages (non-POSIX extension).
    pub const SC_PHYS_PAGES: c_int = 11;
    /// System configuration name for number of available physical pages (non-POSIX extension).
    pub const SC_AVPHYS_PAGES: c_int = 12;
    /// System configuration name for maximum open message queues.
    pub const SC_MQ_OPEN_MAX: c_int = 13;
    /// System configuration name for maximum message queue priority.
    pub const SC_MQ_PRIO_MAX: c_int = 14;
    /// System configuration name for RTSIG_MAX.
    pub const SC_RTSIG_MAX: c_int = 15;
    /// System configuration name for SEM_NSEMS_MAX.
    pub const SC_SEM_NSEMS_MAX: c_int = 16;
    /// System configuration name for SEM_VALUE_MAX.
    pub const SC_SEM_VALUE_MAX: c_int = 17;
    /// System configuration name for SIGQUEUE_MAX.
    pub const SC_SIGQUEUE_MAX: c_int = 18;
    /// System configuration name for TIMER_MAX.
    pub const SC_TIMER_MAX: c_int = 19;
    /// System configuration name for TZNAME_MAX.
    pub const SC_TZNAME_MAX: c_int = 20;
    /// System configuration name for asynchronous I/O.
    pub const SC_ASYNCHRONOUS_IO: c_int = 21;
    /// System configuration name for fsync.
    pub const SC_FSYNC: c_int = 22;
    /// System configuration name for memory mapped files.
    pub const SC_MAPPED_FILES: c_int = 23;
    /// System configuration name for memory locking.
    pub const SC_MEMLOCK: c_int = 24;
    /// System configuration name for memory lock range.
    pub const SC_MEMLOCK_RANGE: c_int = 25;
    /// System configuration name for memory protection.
    pub const SC_MEMORY_PROTECTION: c_int = 26;
    /// System configuration name for message passing.
    pub const SC_MESSAGE_PASSING: c_int = 27;
    /// System configuration name for prioritized I/O.
    pub const SC_PRIORITIZED_IO: c_int = 28;
    /// System configuration name for realtime signals.
    pub const SC_REALTIME_SIGNALS: c_int = 29;
    /// System configuration name for semaphores.
    pub const SC_SEMAPHORES: c_int = 30;
    /// System configuration name for shared memory objects.
    pub const SC_SHARED_MEMORY_OBJECTS: c_int = 31;
    /// System configuration name for synchronized I/O.
    pub const SC_SYNCHRONIZED_IO: c_int = 32;
    /// System configuration name for timers.
    pub const SC_TIMERS: c_int = 33;
    /// System configuration name for AIO listio max.
    pub const SC_AIO_LISTIO_MAX: c_int = 34;
    /// System configuration name for AIO max.
    pub const SC_AIO_MAX: c_int = 35;
    /// System configuration name for AIO priority delta max.
    pub const SC_AIO_PRIO_DELTA_MAX: c_int = 36;
    /// System configuration name for delay timer max.
    pub const SC_DELAYTIMER_MAX: c_int = 37;
    /// System configuration name for thread keys max.
    pub const SC_THREAD_KEYS_MAX: c_int = 38;
    /// System configuration name for thread stack min.
    pub const SC_THREAD_STACK_MIN: c_int = 39;
    /// System configuration name for thread threads max.
    pub const SC_THREAD_THREADS_MAX: c_int = 40;
    /// System configuration name for TTY name max.
    pub const SC_TTY_NAME_MAX: c_int = 41;
    /// System configuration name for threads.
    pub const SC_THREADS: c_int = 42;
    /// System configuration name for thread attr stack address.
    pub const SC_THREAD_ATTR_STACKADDR: c_int = 43;
    /// System configuration name for thread attr stack size.
    pub const SC_THREAD_ATTR_STACKSIZE: c_int = 44;
    /// System configuration name for thread priority scheduling.
    pub const SC_THREAD_PRIORITYSCHEDULING: c_int = 45;
    /// System configuration name for thread priority inherit.
    pub const SC_THREAD_PRIO_INHERIT: c_int = 46;
    /// System configuration name for thread priority protect.
    pub const SC_THREAD_PRIO_PROTECT: c_int = 47;
    /// Alias of SC_THREAD_PRIO_PROTECT.
    pub const SC_THREAD_PRIO_CEILING: c_int = SC_THREAD_PRIO_PROTECT;
    /// System configuration name for thread process shared.
    pub const SC_THREAD_PROCESS_SHARED: c_int = 48;
    /// System configuration name for thread safe functions.
    pub const SC_THREAD_SAFE_FUNCTIONS: c_int = 49;
    /// System configuration name for getgr_r size max.
    pub const SC_GETGR_R_SIZE_MAX: c_int = 50;
    /// System configuration name for getpw_r size max.
    pub const SC_GETPW_R_SIZE_MAX: c_int = 51;
    /// System configuration name for login name max.
    pub const SC_LOGIN_NAME_MAX: c_int = 52;
    /// System configuration name for thread destructor iterations.
    pub const SC_THREAD_DESTRUCTOR_ITERATIONS: c_int = 53;
    /// System configuration name for advisory info.
    pub const SC_ADVISORY_INFO: c_int = 54;
    /// System configuration name for atexit max.
    pub const SC_ATEXIT_MAX: c_int = 55;
    /// System configuration name for barriers.
    pub const SC_BARRIERS: c_int = 56;
    /// System configuration name for bc base max.
    pub const SC_BC_BASE_MAX: c_int = 57;
    /// System configuration name for bc dim max.
    pub const SC_BC_DIM_MAX: c_int = 58;
    /// System configuration name for bc scale max.
    pub const SC_BCSCALE_MAX: c_int = 59;
    /// System configuration name for bc string max.
    pub const SC_BC_STRING_MAX: c_int = 60;
    /// System configuration name for clock selection.
    pub const SC_CLOCK_SELECTION: c_int = 61;
    /// System configuration name for coll weights max.
    pub const SC_COLL_WEIGHTS_MAX: c_int = 62;
    /// System configuration name for cputime.
    pub const SC_CPUTIME: c_int = 63;
    /// System configuration name for expr nest max.
    pub const SC_EXPR_NEST_MAX: c_int = 64;
    /// System configuration name for host name max.
    pub const SC_HOST_NAME_MAX: c_int = 65;
    /// System configuration name for iov max.
    pub const SC_IOV_MAX: c_int = 66;
    /// System configuration name for IPv6 support.
    pub const SC_IPV6: c_int = 67;
    /// System configuration name for line max.
    pub const SC_LINE_MAX: c_int = 68;
    /// System configuration name for monotonic clock.
    pub const SC_MONOTONIC_CLOCK: c_int = 69;
    /// System configuration name for raw sockets.
    pub const SC_RAW_SOCKETS: c_int = 70;
    /// System configuration name for reader writer locks.
    pub const SC_READER_WRITER_LOCKS: c_int = 71;
    /// System configuration name for regexp.
    pub const SC_REGEXP: c_int = 72;
    /// System configuration name for re duplicate max.
    pub const SC_RE_DUP_MAX: c_int = 73;
    /// System configuration name for shell.
    pub const SC_SHELL: c_int = 74;
    /// System configuration name for spawn.
    pub const SC_SPAWN: c_int = 75;
    /// System configuration name for spin locks.
    pub const SC_SPIN_LOCKS: c_int = 76;
    /// System configuration name for sporadic server.
    pub const SC_SPORADIC_SERVER: c_int = 77;
    /// System configuration name for ss repl max.
    pub const SC_SS_REPL_MAX: c_int = 78;
    /// System configuration name for symlink loop max.
    pub const SC_SYMLOOP_MAX: c_int = 79;
    /// System configuration name for thread cputime.
    pub const SC_THREAD_CPUTIME: c_int = 80;
    /// System configuration name for thread sporadic server.
    pub const SC_THREAD_SPORADIC_SERVER: c_int = 81;
    /// System configuration name for timeouts.
    pub const SC_TIMEOUTS: c_int = 82;
    /// System configuration name for trace.
    pub const SC_TRACE: c_int = 83;
    /// System configuration name for trace event filter.
    pub const SC_TRACE_EVENT_FILTER: c_int = 84;
    /// System configuration name for trace event name max.
    pub const SC_TRACE_EVENT_NAME_MAX: c_int = 85;
    /// System configuration name for trace inherit.
    pub const SC_TRACE_INHERIT: c_int = 86;
    /// System configuration name for trace log.
    pub const SC_TRACE_LOG: c_int = 87;
    /// System configuration name for trace name max.
    pub const SC_TRACE_NAME_MAX: c_int = 88;
    /// System configuration name for trace sys max.
    pub const SC_TRACE_SYS_MAX: c_int = 89;
    /// System configuration name for trace user event max.
    pub const SC_TRACE_USER_EVENT_MAX: c_int = 90;
    /// System configuration name for typed memory objects.
    pub const SC_TYPED_MEMORY_OBJECTS: c_int = 91;
    /// System configuration name for V7 ILP32 OFF32.
    pub const SC_V7_ILP32_OFF32: c_int = 92;
    /// Alias of SC_V7_ILP32_OFF32.
    pub const SC_V6_ILP32_OFF32: c_int = SC_V7_ILP32_OFF32;
    /// Alias of SC_V7_ILP32_OFF32.
    pub const SC_XBS5_ILP32_OFF32: c_int = SC_V7_ILP32_OFF32;
    /// System configuration name for V7 ILP32 OFFBIG.
    pub const SC_V7_ILP32_OFFBIG: c_int = 93;
    /// Alias of SC_V7_ILP32_OFFBIG.
    pub const SC_V6_ILP32_OFFBIG: c_int = SC_V7_ILP32_OFFBIG;
    /// Alias of SC_V7_ILP32_OFFBIG.
    pub const SC_XBS5_ILP32_OFFBIG: c_int = SC_V7_ILP32_OFFBIG;
    /// System configuration name for V7 LP64 OFF64.
    pub const SC_V7_LP64_OFF64: c_int = 94;
    /// Alias of SC_V7_LP64_OFF64.
    pub const SC_V6_LP64_OFF64: c_int = SC_V7_LP64_OFF64;
    /// Alias of SC_V7_LP64_OFF64.
    pub const SC_XBS5_LP64_OFF64: c_int = SC_V7_LP64_OFF64;
    /// System configuration name for V7 LPBIG OFFBIG.
    pub const SC_V7_LPBIG_OFFBIG: c_int = 95;
    /// Alias of SC_V7_LPBIG_OFFBIG.
    pub const SC_V6_LPBIG_OFFBIG: c_int = SC_V7_LPBIG_OFFBIG;
    /// Alias of SC_V7_LPBIG_OFFBIG.
    pub const SC_XBS5_LPBIG_OFFBIG: c_int = SC_V7_LPBIG_OFFBIG;
    /// System configuration name for XOPEN crypt.
    pub const SC_XOPEN_CRYPT: c_int = 96;
    /// System configuration name for XOPEN enhanced i18n.
    pub const SC_XOPEN_ENH_I18N: c_int = 97;
    /// System configuration name for XOPEN legacy.
    pub const SC_XOPEN_LEGACY: c_int = 98;
    /// System configuration name for XOPEN realtime.
    pub const SC_XOPEN_REALTIME: c_int = 99;
    /// System configuration name for stream max.
    pub const SC_STREAM_MAX: c_int = 100;
    /// System configuration name for priority scheduling.
    pub const SC_PRIORITYSCHEDULING: c_int = 101;
    /// System configuration name for XOPEN realtime threads.
    pub const SC_XOPEN_REALTIME_THREADS: c_int = 102;
    /// System configuration name for XOPEN shared memory.
    pub const SC_XOPEN_SHM: c_int = 103;
    /// System configuration name for XOPEN streams.
    pub const SC_XOPEN_STREAMS: c_int = 104;
    /// System configuration name for XOPEN UNIX.
    pub const SC_XOPEN_UNIX: c_int = 105;
    /// System configuration name for XOPEN version.
    pub const SC_XOPEN_VERSION: c_int = 106;
    /// System configuration name for POSIX.2 char term.
    pub const SC_2_CHAR_TERM: c_int = 107;
    /// System configuration name for POSIX.2 C bindings.
    pub const SC_2_C_BIND: c_int = 108;
    /// System configuration name for POSIX.2 C development.
    pub const SC_2_C_DEV: c_int = 109;
    /// System configuration name for POSIX.2 Fortran development.
    pub const SC_2_FORT_DEV: c_int = 110;
    /// System configuration name for POSIX.2 Fortran runtime.
    pub const SC_2_FORT_RUN: c_int = 111;
    /// System configuration name for POSIX.2 localedef.
    pub const SC_2_LOCALEDEF: c_int = 112;
    /// System configuration name for POSIX.2 batch system.
    pub const SC_2_PBS: c_int = 113;
    /// System configuration name for POSIX.2 batch accounting.
    pub const SC_2_PBS_ACCOUNTING: c_int = 114;
    /// System configuration name for POSIX.2 batch checkpoint.
    pub const SC_2_PBS_CHECKPOINT: c_int = 115;
    /// System configuration name for POSIX.2 batch locate.
    pub const SC_2_PBS_LOCATE: c_int = 116;
    /// System configuration name for POSIX.2 batch message.
    pub const SC_2_PBS_MESSAGE: c_int = 117;
    /// System configuration name for POSIX.2 batch track.
    pub const SC_2_PBS_TRACK: c_int = 118;
    /// System configuration name for POSIX.2 software dev.
    pub const SC_2_SW_DEV: c_int = 119;
    /// System configuration name for POSIX.2 UPE.
    pub const SC_2_UPE: c_int = 120;
    /// System configuration name for POSIX.2 version.
    pub const SC_2_VERSION: c_int = 121;
    /// System configuration name for thread robust priority inherit.
    pub const SC_THREAD_ROBUST_PRIO_INHERIT: c_int = 122;
    /// System configuration name for thread robust priority protect.
    pub const SC_THREAD_ROBUST_PRIO_PROTECT: c_int = 123;
    /// System configuration name for XOPEN UUCP.
    pub const SC_XOPEN_UUCP: c_int = 124;
    /// System configuration name for level1 instruction cache size (non-POSIX extension).
    pub const SC_LEVEL1_ICACHE_SIZE: c_int = 125;
    /// System configuration name for level1 instruction cache associativity (non-POSIX extension).
    pub const SC_LEVEL1_ICACHE_ASSOC: c_int = 126;
    /// System configuration name for level1 instruction cache line size (non-POSIX extension).
    pub const SC_LEVEL1_ICACHE_LINESIZE: c_int = 127;
    /// System configuration name for level1 data cache size (non-POSIX extension).
    pub const SC_LEVEL1_DCACHE_SIZE: c_int = 128;
    /// System configuration name for level1 data cache associativity (non-POSIX extension).
    pub const SC_LEVEL1_DCACHE_ASSOC: c_int = 129;
    /// System configuration name for level1 data cache line size (non-POSIX extension).
    pub const SC_LEVEL1_DCACHE_LINESIZE: c_int = 130;
    /// System configuration name for level2 cache size (non-POSIX extension).
    pub const SC_LEVEL2_CACHE_SIZE: c_int = 131;
    /// System configuration name for level2 cache associativity (non-POSIX extension).
    pub const SC_LEVEL2_CACHE_ASSOC: c_int = 132;
    /// System configuration name for level2 cache line size (non-POSIX extension).
    pub const SC_LEVEL2_CACHE_LINESIZE: c_int = 133;
    /// System configuration name for level3 cache size (non-POSIX extension).
    pub const SC_LEVEL3_CACHE_SIZE: c_int = 134;
    /// System configuration name for level3 cache associativity (non-POSIX extension).
    pub const SC_LEVEL3_CACHE_ASSOC: c_int = 135;
    /// System configuration name for level3 cache line size (non-POSIX extension).
    pub const SC_LEVEL3_CACHE_LINESIZE: c_int = 136;
    /// System configuration name for level4 cache size (non-POSIX extension).
    pub const SC_LEVEL4_CACHE_SIZE: c_int = 137;
    /// System configuration name for level4 cache associativity (non-POSIX extension).
    pub const SC_LEVEL4_CACHE_ASSOC: c_int = 138;
    /// System configuration name for level4 cache line size (non-POSIX extension).
    pub const SC_LEVEL4_CACHE_LINESIZE: c_int = 139;
    /// System configuration name for POSIX.26 version (non-POSIX extension).
    pub const SC_POSIX_26_VERSION: c_int = 140;
}

//==================================================================================================
// Function Prototypes
//==================================================================================================

unsafe extern "C" {
    pub unsafe fn getentropy(_buffer: *mut c_void, _length: size_t) -> c_int;
}
