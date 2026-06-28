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
        c_long,
        c_longlong,
        c_uint,
        c_ulong,
        c_ulonglong,
        c_ushort,
        c_void,
    },
    pthread::pthread_mutex_type::PTHREAD_MUTEX_DEFAULT,
    sched::{
        sched_param,
        sched_policy::SCHED_OTHER,
    },
    sys_socket::socklen_t,
};
use ::config::memory_layout::{
    USER_STACK_TOP_RAW,
    USER_THREAD_STACK_SIZE,
};
use ::core::mem::size_of;

use crate::sys_uio::iovec;

//==================================================================================================
// Types
//==================================================================================================

/// Used for file block counts.
pub type blkcnt_t = c_longlong;

/// Used for block sizes.
pub type blksize_t = c_longlong;

/// Used for system times in clock ticks or `CLOCKS_PER_SEC`.
pub type clock_t = c_longlong;

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = c_int;

/// Used for device IDs.
pub type dev_t = c_ulonglong;

/// Used for file-system block counts.
pub type fsblkcnt_t = c_ulong;

/// Used for file-system file (node) counts.
pub type fsfilcnt_t = c_ulong;

/// Used for group IDs.
pub type gid_t = c_uint;

/// Used for file serial numbers.
pub type ino_t = c_ulonglong;

/// Used for file attributes.
pub type mode_t = c_uint;

/// Used for link counts.
pub type nlink_t = c_ulonglong;

/// Used for file sizes.
pub type off_t = c_longlong;

/// Used for process IDs and process group IDs.
pub type pid_t = c_int;

/// Used to identify a thread.
pub type pthread_t = u32;

/// Used for condition variables.
pub type pthread_cond_t = u32;

/// Used for thread-specific data keys.
pub type pthread_key_t = u32;

/// Used for mutexes.
pub type pthread_mutex_t = u32;

/// Used for read-write locks.
pub type pthread_rwlock_t = u32;

/// Used for directory entry lengths.
pub type reclen_t = c_ushort;

/// Used for object sizes.
pub type c_size_t = c_uint;

/// Used for object sizes (architecture dependent).
pub type size_t = usize;

/// Used for a count of bytes or an error indication.
pub type c_ssize_t = c_int;

/// Used for a count of bytes or an error indication (architecture dependent).
pub type ssize_t = isize;

/// Used for time in microseconds.
pub type suseconds_t = c_long;

/// Used for time in seconds.
pub type time_t = c_longlong;

/// Used for user IDs.
pub type uid_t = c_uint;

//==================================================================================================
// Structures
//==================================================================================================

/// Thread attributes.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct pthread_attr_t {
    pub is_initialized: c_int,
    pub stackaddr: *mut c_void,
    pub stacksize: c_size_t,
    pub contentionscope: c_int,
    pub inheritsched: c_int,
    pub schedpolicy: c_int,
    pub schedparam: sched_param,
    pub cputime_clock_allowed: c_int,
    pub detachstate: c_int,
}
// No `assert_eq_size!`: the serialized size may differ from `sizeof` on 64-bit targets due to
// alignment padding.

impl pthread_attr_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = size_of::<c_int>();
    /// Size of the `stackaddr` field.
    const SIZE_OF_STACKADDR: usize = size_of::<*mut c_void>();
    /// Size of the `stacksize` field.
    const SIZE_OF_STACKSIZE: usize = size_of::<c_size_t>();
    /// Size of the `contentionscope` field.
    const SIZE_OF_CONTENTIONSCOPE: usize = size_of::<c_int>();
    /// Size of the `inheritsched` field.
    const SIZE_OF_INHERITSCHED: usize = size_of::<c_int>();
    /// Size of the `schedpolicy` field.
    const SIZE_OF_SCHEDPOLICY: usize = size_of::<c_int>();
    /// Size of the `schedparam` field.
    const SIZE_OF_SCHEDPARAM: usize = size_of::<sched_param>();
    /// Size of the `cputime_clock_allowed` field.
    const SIZE_OF_CPUTIME_CLOCK_ALLOWED: usize = size_of::<c_int>();
    /// Size of the `detachstate` field.
    const SIZE_OF_DETACHSTATE: usize = size_of::<c_int>();

    /// Size of this structure.
    pub const _SIZE: usize = Self::SIZE_OF_IS_INITIALIZED
        + Self::SIZE_OF_STACKADDR
        + Self::SIZE_OF_STACKSIZE
        + Self::SIZE_OF_CONTENTIONSCOPE
        + Self::SIZE_OF_INHERITSCHED
        + Self::SIZE_OF_SCHEDPOLICY
        + Self::SIZE_OF_SCHEDPARAM
        + Self::SIZE_OF_CPUTIME_CLOCK_ALLOWED
        + Self::SIZE_OF_DETACHSTATE;
}

impl Default for pthread_attr_t {
    fn default() -> Self {
        // TODO: review this once all fields are supported
        Self {
            is_initialized: 1,
            stackaddr: USER_STACK_TOP_RAW as *mut _,
            stacksize: USER_THREAD_STACK_SIZE as c_size_t,
            contentionscope: 0,
            inheritsched: 0,
            schedpolicy: SCHED_OTHER,
            schedparam: sched_param::default(),
            cputime_clock_allowed: 0,
            detachstate: 0,
        }
    }
}

///
/// # Description
///
/// Condition variable attributes.
///
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct pthread_condattr_t {
    /// Whether the condition variable attributes are initialized.
    is_initialized: c_int,
    /// Clock used for timeouts.
    clock: clock_t,
}
// No `assert_eq_size!`: the serialized size may differ from `sizeof` on 64-bit targets due to
// alignment padding.

impl pthread_condattr_t {
    // Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = size_of::<c_int>();
    // Size of the `clock` field.
    const SIZE_OF_CLOCK: usize = size_of::<clock_t>();

    /// Size of `pthread_condattr_t` structure.
    pub const SIZE: usize = Self::SIZE_OF_IS_INITIALIZED + Self::SIZE_OF_CLOCK;
}

impl Default for pthread_condattr_t {
    fn default() -> Self {
        Self {
            is_initialized: 1,
            clock: 0,
        }
    }
}

///
/// # Description
///
/// Mutex attributes.
///
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct pthread_mutexattr_t {
    /// Whether the mutex attributes are initialized.
    is_initialized: c_int,
    /// Type of mutex.
    type_: c_int,
    /// Whether the mutex is recursive.
    recursive: c_int,
}
::static_assert::assert_eq_size!(pthread_mutexattr_t, pthread_mutexattr_t::SIZE);

impl pthread_mutexattr_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = size_of::<c_int>();
    /// Size of the `type_` field.
    const SIZE_OF_TYPE: usize = size_of::<c_int>();
    /// Size of the `recursive` field.
    const SIZE_OF_RECURSIVE: usize = size_of::<c_int>();

    /// Size of `pthread_mutexattr_t` structure.
    pub const SIZE: usize =
        Self::SIZE_OF_IS_INITIALIZED + Self::SIZE_OF_TYPE + Self::SIZE_OF_RECURSIVE;

    /// Returns the mutex type stored in the attributes object.
    pub fn type_(&self) -> c_int {
        self.type_
    }

    /// Sets the mutex type stored in the attributes object.
    pub fn set_type(&mut self, type_: c_int) {
        self.type_ = type_;
        self.recursive = (type_ == crate::pthread::pthread_mutex_type::PTHREAD_MUTEX_RECURSIVE)
            as crate::ffi::c_int;
    }
}

impl Default for pthread_mutexattr_t {
    fn default() -> Self {
        Self {
            is_initialized: 1,
            type_: PTHREAD_MUTEX_DEFAULT,
            recursive: 0,
        }
    }
}

///
/// # Description
///
/// Read-write lock attributes.
///
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct pthread_rwlockattr_t {
    /// Whether the read-write lock attributes are initialized.
    is_initialized: c_int,
}
::static_assert::assert_eq_size!(pthread_rwlockattr_t, pthread_rwlockattr_t::SIZE);

impl pthread_rwlockattr_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = size_of::<c_int>();

    /// Size of `pthread_rwlockattr_t` structure.
    pub const SIZE: usize = Self::SIZE_OF_IS_INITIALIZED;
}

impl Default for pthread_rwlockattr_t {
    fn default() -> Self {
        Self { is_initialized: 1 }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct pthread_once_t {
    /// Whether the `pthread_once` is initialized.
    is_initialized: c_int,
    /// Whether the `pthread_once` has been executed.
    init_executed: c_int,
}
::static_assert::assert_eq_size!(pthread_once_t, pthread_once_t::SIZE);

impl pthread_once_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = size_of::<c_int>();
    /// Size of the `init_executed` field.
    const SIZE_OF_INIT_EXECUTED: usize = size_of::<c_int>();

    /// Size of `pthread_once_t` structure.
    pub const SIZE: usize = Self::SIZE_OF_IS_INITIALIZED + Self::SIZE_OF_INIT_EXECUTED;

    /// Sentinel value of `is_initialized` set by `PTHREAD_ONCE_INIT`.
    pub const IS_INITIALIZED_VALUE: c_int = 1;

    /// Statically-initialized `pthread_once_t`, equivalent to POSIX `PTHREAD_ONCE_INIT`.
    ///
    /// `is_initialized` is set to the sentinel recognized by `pthread_once()`, and `init_executed`
    /// is `0`, meaning the initializer has not yet run (the `ONCE_NEVER_RUN` state).
    pub const INIT: pthread_once_t = pthread_once_t {
        is_initialized: Self::IS_INITIALIZED_VALUE,
        init_executed: 0,
    };

    ///
    /// # Description
    ///
    /// Reads the `is_initialized` field through a raw pointer without constructing a Rust reference
    /// to `pthread_once_t`.
    ///
    /// `is_initialized` is set to `1` by `PTHREAD_ONCE_INIT`.  A non-`1` value indicates that the
    /// caller forgot to use `PTHREAD_ONCE_INIT` and the `pthread_once_t` is uninitialized.
    ///
    /// This is provided as an associated function on `*const Self` (rather than `&self`) so callers
    /// like `pthread_once()` can inspect the field without ever materialising a Rust reference.
    /// Materialising a `&mut pthread_once_t` and then re-entering `pthread_once()` recursively on
    /// the same control word would create a second `&mut` to the same object, which is
    /// Stacked-Borrows UB even though the implementation explicitly handles the recursive case.
    ///
    /// # Parameters
    ///
    /// `once` - A pointer to the `pthread_once_t` to read.
    ///
    /// # Returns
    ///
    /// The value of the `is_initialized` field, which is `1` if the `pthread_once_t` is initialized
    /// and a non-`1` value if it is uninitialized.
    ///
    /// # Safety
    ///
    /// `once` must be non-null and point to a valid `pthread_once_t`.
    ///
    pub unsafe fn is_initialized_raw(once: *const pthread_once_t) -> c_int {
        // SAFETY: caller guarantees `once` is valid.  `read_unaligned` is used because
        // `pthread_once_t` is `#[repr(C, packed)]`, so `addr_of!` produces an alignment-1 pointer
        // from Rust's perspective.
        unsafe { ::core::ptr::addr_of!((*once).is_initialized).read_unaligned() }
    }

    ///
    /// # Description
    ///
    /// Returns a mutable raw pointer to the `init_executed` field without constructing a Rust
    /// reference to `pthread_once_t`.
    ///
    /// `pthread_once()` uses this pointer with `ptr::read_unaligned` / `ptr::write_unaligned` to
    /// implement the state-machine transitions without the compiler optimising the loads or stores
    /// away.  Unaligned accessors are used because `pthread_once_t` is `#[repr(C, packed)]`, so
    /// this pointer has alignment 1 from Rust's perspective even though the underlying address is
    /// usually naturally aligned; `volatile` reads/writes would be UB on a (potentially) unaligned
    /// pointer.  Concurrent transitions are serialized by the process-global kernel mutex that
    /// `pthread_once()` holds while it reads and writes this word, so plain unaligned loads and
    /// stores -- rather than atomics -- are sufficient here.
    ///
    /// See `is_initialized_raw()` for why this is a raw-pointer function rather than a method on
    /// `&mut self`.
    ///
    /// # Parameters
    ///
    /// - `once` - A pointer to the `pthread_once_t` to access.
    ///
    /// # Returns
    ///
    /// A mutable raw pointer to the `init_executed` field of the `pthread_once_t`.
    ///
    /// # Safety
    ///
    /// `once` must be non-null and point to a valid `pthread_once_t`
    /// whose lifetime exceeds the use of the returned pointer.
    ///
    pub unsafe fn init_executed_ptr_raw(once: *mut pthread_once_t) -> *mut c_int {
        // SAFETY: caller guarantees `once` is valid.  No Rust reference is constructed.
        unsafe { ::core::ptr::addr_of_mut!((*once).init_executed) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
#[cfg(target_pointer_width = "32")]
pub struct msghdr {
    /// Optional address.
    pub msg_name: *mut c_void,
    // Size of the address.
    pub msg_namelen: socklen_t,
    // Scatter/gather array of message blocks
    pub msg_iov: *mut iovec,
    /// Number of member in `msg_iov`.
    pub msg_iovlen: c_int,
    /// Ancillary data.
    pub msg_control: *mut c_void,
    /// Ancillary data buffer length.
    pub msg_controllen: socklen_t,
    /// Flags.
    pub msg_flags: c_int,
}
#[cfg(target_pointer_width = "32")]
::static_assert::assert_eq_size!(msghdr, msghdr::SIZE);

#[cfg(target_pointer_width = "32")]
impl msghdr {
    /// Size of the `msg_name` field.
    const SIZE_OF_MSG_NAME: usize = size_of::<*mut c_void>();
    /// Size of the `msg_namelen` field.
    const SIZE_OF_MSG_NAMELEN: usize = size_of::<socklen_t>();
    /// Size of the `msg_iov` field.
    const SIZE_OF_MSG_IOV: usize = size_of::<*mut iovec>();
    /// Size of the `msg_iovlen` field.
    const SIZE_OF_MSG_IOVLEN: usize = size_of::<c_int>();
    /// Size of the `msg_control` field.
    const SIZE_OF_MSG_CONTROL: usize = size_of::<*mut c_void>();
    /// Size of the `msg_controllen` field.
    const SIZE_OF_MSG_CONTROLLEN: usize = size_of::<socklen_t>();
    /// Size of the `msg_flags` field.
    const SIZE_OF_MSG_FLAGS: usize = size_of::<c_int>();

    /// Size of `msghdr` structure.
    pub const SIZE: usize = Self::SIZE_OF_MSG_NAME
        + Self::SIZE_OF_MSG_NAMELEN
        + Self::SIZE_OF_MSG_IOV
        + Self::SIZE_OF_MSG_IOVLEN
        + Self::SIZE_OF_MSG_CONTROL
        + Self::SIZE_OF_MSG_CONTROLLEN
        + Self::SIZE_OF_MSG_FLAGS;
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
#[cfg(target_pointer_width = "64")]
pub struct msghdr {
    /// Optional address.
    pub msg_name: *mut c_void,
    // Size of the address.
    pub msg_namelen: socklen_t,
    // Scatter/gather array of message blocks.
    pub msg_iov: *mut iovec,
    /// Number of member in `msg_iov`.
    pub msg_iovlen: size_t,
    /// Ancillary data.
    pub msg_control: *mut c_void,
    /// Ancillary data buffer length.
    pub msg_controllen: size_t,
    /// Flags.
    pub msg_flags: c_int,
}

/// Header for ancililary data data objects in msg_control buffer in `msghdr`.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct cmsghdr {
    /// Data byte count, including the control message header..
    pub cmsg_len: socklen_t,
    /// Originating protocol.
    pub cmsg_level: c_int,
    /// Protocol-specific type.
    pub cmsg_type: c_int,
}
::static_assert::assert_eq_size!(cmsghdr, cmsghdr::SIZE);

impl cmsghdr {
    /// Size of the `cmsg_len` field.
    const SIZE_OF_CMSG_LEN: usize = size_of::<socklen_t>();
    /// Size of the `cmsg_level` field.
    const SIZE_OF_CMSG_LEVEL: usize = size_of::<c_int>();
    /// Size of the `cmsg_type` field.
    const SIZE_OF_CMSG_TYPE: usize = size_of::<c_int>();

    /// Size of `cmsghdr` structure.
    pub const SIZE: usize =
        Self::SIZE_OF_CMSG_LEN + Self::SIZE_OF_CMSG_LEVEL + Self::SIZE_OF_CMSG_TYPE;
}
