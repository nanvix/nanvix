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
    sched::{
        self,
        sched_param,
    },
    sys::{
        socket::socklen_t,
        uio::iovec,
    },
};
use ::core::mem;

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

/// Used for directory entry lengths.
pub type reclen_t = c_ushort;

/// Used for object sizes.
pub type size_t = c_uint;

/// Used for a count of bytes or an error indication.
pub type ssize_t = c_int;

/// Used for time in microseconds.
pub type suseconds_t = c_long;

/// Used for time in seconds.
pub type time_t = c_longlong;

/// Used for user IDs.
pub type uid_t = c_uint;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Thread attributes.
///
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct pthread_attr_t {
    pub is_initialized: c_int,
    pub stackaddr: *mut c_void,
    pub stacksize: size_t,
    pub contentionscope: c_int,
    pub inheritsched: c_int,
    pub schedpolicy: c_int,
    pub schedparam: sched_param,
    pub cputime_clock_allowed: c_int,
    pub detachstate: c_int,
}
::static_assert::assert_eq_size!(pthread_attr_t, pthread_attr_t::SIZE);

impl pthread_attr_t {
    /// Size of the `is_initialized` field.
    const SIZE_OF_IS_INITIALIZED: usize = core::mem::size_of::<c_int>();
    /// Size of the `stackaddr` field.
    const SIZE_OF_STACKADDR: usize = core::mem::size_of::<*mut c_void>();
    /// Size of the `stacksize` field.
    const SIZE_OF_STACKSIZE: usize = core::mem::size_of::<size_t>();
    /// Size of the `contentionscope` field.
    const SIZE_OF_CONTENTIONSCOPE: usize = core::mem::size_of::<c_int>();
    /// Size of the `inheritsched` field.
    const SIZE_OF_INHERITSCHED: usize = core::mem::size_of::<c_int>();
    /// Size of the `schedpolicy` field.
    const SIZE_OF_SCHEDPOLICY: usize = core::mem::size_of::<c_int>();
    /// Size of the `schedparam` field.
    const SIZE_OF_SCHEDPARAM: usize = core::mem::size_of::<sched_param>();
    /// Size of the `cputime_clock_allowed` field.
    const SIZE_OF_CPUTIME_CLOCK_ALLOWED: usize = core::mem::size_of::<c_int>();
    /// Size of the `detachstate` field.
    const SIZE_OF_DETACHSTATE: usize = core::mem::size_of::<c_int>();

    /// Size of `pthread_attr_t` structure.
    pub const SIZE: usize = Self::SIZE_OF_IS_INITIALIZED
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
            stackaddr: core::ptr::null_mut(),
            stacksize: 0,
            contentionscope: 0,
            inheritsched: 0,
            schedpolicy: sched::sched_policy::SCHED_OTHER,
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
    const SIZE_OF_IS_INITIALIZED: usize = mem::size_of::<c_int>();
    // Size of the `clock` field.
    const SIZE_OF_CLOCK: usize = mem::size_of::<clock_t>();

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
    const SIZE_OF_IS_INITIALIZED: usize = mem::size_of::<c_int>();
    /// Size of the `init_executed` field.
    const SIZE_OF_INIT_EXECUTED: usize = mem::size_of::<c_int>();

    /// Size of `pthread_once_t` structure.
    pub const SIZE: usize = Self::SIZE_OF_IS_INITIALIZED + Self::SIZE_OF_INIT_EXECUTED;
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
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
::static_assert::assert_eq_size!(msghdr, msghdr::SIZE);

impl msghdr {
    /// Size of the `msg_name` field.
    const SIZE_OF_MSG_NAME: usize = mem::size_of::<*mut c_void>();
    /// Size of the `msg_namelen` field.
    const SIZE_OF_MSG_NAMELEN: usize = mem::size_of::<socklen_t>();
    /// Size of the `msg_iov` field.
    const SIZE_OF_MSG_IOV: usize = mem::size_of::<*mut iovec>();
    /// Size of the `msg_iovlen` field.
    const SIZE_OF_MSG_IOVLEN: usize = mem::size_of::<c_int>();
    /// Size of the `msg_control` field.
    const SIZE_OF_MSG_CONTROL: usize = mem::size_of::<*mut c_void>();
    /// Size of the `msg_controllen` field.
    const SIZE_OF_MSG_CONTROLLEN: usize = mem::size_of::<socklen_t>();
    /// Size of the `msg_flags` field.
    const SIZE_OF_MSG_FLAGS: usize = mem::size_of::<c_int>();

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
    const SIZE_OF_CMSG_LEN: usize = mem::size_of::<socklen_t>();
    /// Size of the `cmsg_level` field.
    const SIZE_OF_CMSG_LEVEL: usize = mem::size_of::<c_int>();
    /// Size of the `cmsg_type` field.
    const SIZE_OF_CMSG_TYPE: usize = mem::size_of::<c_int>();

    /// Size of `cmsghdr` structure.
    pub const SIZE: usize =
        Self::SIZE_OF_CMSG_LEN + Self::SIZE_OF_CMSG_LEVEL + Self::SIZE_OF_CMSG_TYPE;
}
