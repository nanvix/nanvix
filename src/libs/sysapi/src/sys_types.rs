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
        c_ulonglong,
        c_ushort,
        c_void,
    },
    pthread::pthread_mutex_type::PTHREAD_MUTEX_NORMAL,
    sched::{
        sched_param,
        sched_policy::SCHED_OTHER,
    },
    sys_socket::socklen_t,
};
use ::config::memory_layout::{
    USER_STACK_SIZE,
    USER_STACK_TOP_RAW,
};
use ::core::mem::size_of;

#[cfg(target_pointer_width = "32")]
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
#[repr(C, packed)]
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
::static_assert::assert_eq_size!(pthread_attr_t, pthread_attr_t::_SIZE);

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
            stacksize: USER_STACK_SIZE as c_size_t,
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
#[repr(C, packed)]
pub struct pthread_condattr_t {
    /// Whether the condition variable attributes are initialized.
    is_initialized: c_int,
    /// Clock used for timeouts.
    clock: clock_t,
}
::static_assert::assert_eq_size!(pthread_condattr_t, pthread_condattr_t::SIZE);

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
}

impl Default for pthread_mutexattr_t {
    fn default() -> Self {
        Self {
            is_initialized: 1,
            type_: PTHREAD_MUTEX_NORMAL,
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
