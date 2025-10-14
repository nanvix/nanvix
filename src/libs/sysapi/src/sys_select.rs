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
    ffi::c_ulong,
    sys_types::{
        suseconds_t,
        time_t,
    },
};
use ::core::mem::size_of;

//==================================================================================================
// Constants
//==================================================================================================

/// Microseconds in a second.
const MICROSECONDS_PER_SECOND: i32 = 1_000_000;

/// Nanoseconds in a second.
const NANOSECONDS_PER_SECOND: i32 = 1_000_000_000;

/// Maximum number of file descriptors tracked by [`fd_set`].
pub const FD_SETSIZE: usize = 64;

/// Number of bits per file descriptor mask word.
const FD_SET_WORD_BITS: usize = c_ulong::BITS as usize;

/// Number of words required to represent [`FD_SETSIZE`] file descriptors.
const FD_SET_WORD_COUNT: usize = FD_SETSIZE.div_ceil(FD_SET_WORD_BITS);

//==================================================================================================
// Enumerations
//==================================================================================================

/// Errors that can occur when operating on an [`fd_set`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FdSetError {
    /// File descriptor index is outside the tracked range.
    FileDescriptorOutOfRange,
}

impl ::core::fmt::Display for FdSetError {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            Self::FileDescriptorOutOfRange => {
                formatter.write_str("file descriptor index is out of range")
            },
        }
    }
}

//==================================================================================================
// Structures
//==================================================================================================

/// Bit mask set used by `select()` to represent file descriptors of interest.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct fd_set {
    /// Bit fields for tracked file descriptors.
    pub fds_bits: [c_ulong; FD_SET_WORD_COUNT],
}
::static_assert::assert_eq_size!(fd_set, size_of::<[c_ulong; FD_SET_WORD_COUNT]>());

impl fd_set {
    ///
    /// # Description
    ///
    /// Computes the position of a file descriptor in a file descriptor set.
    ///
    /// # Parameters
    ///
    /// - `fd`: Target file descriptor.
    ///
    /// # Return Value
    ///
    /// This function returns a tuple containing:
    /// - The index of the word in the `fds_bits` array where the bit for the specified file
    ///   descriptor is located.
    /// - The bitmask with the bit for the specified file descriptor set.
    ///
    fn bit_position(fd: usize) -> (usize, c_ulong) {
        let word_index: usize = fd / FD_SET_WORD_BITS;
        let bit_offset: usize = fd % FD_SET_WORD_BITS;
        let mask: c_ulong = 1 << (bit_offset as u32);
        (word_index, mask)
    }

    ///
    /// # Description
    ///
    /// Checks if a specific file descriptor is set in the fd_set.
    ///
    /// # Parameters
    ///
    /// - `fd`: The file descriptor number to check
    ///
    /// # Returns
    ///
    /// This function returns `Ok(true)` if the file descriptor is set, `Ok(false)` if it is not
    /// set, or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of range
    /// (`fd >= FD_SETSIZE`).
    ///
    pub fn is_set(&self, fd: usize) -> Result<bool, FdSetError> {
        if fd >= FD_SETSIZE {
            return Err(FdSetError::FileDescriptorOutOfRange);
        }
        let (word_index, mask): (usize, c_ulong) = Self::bit_position(fd);
        // SAFETY: Access through raw pointer to avoid creating an unaligned reference to a packed field.
        let base: *const c_ulong = ::core::ptr::addr_of!(self.fds_bits) as *const c_ulong;
        let word: c_ulong = unsafe { base.add(word_index).read() };
        Ok(word & mask != 0)
    }

    ///
    /// # Description
    ///
    /// Raw pointer version of `is_set()`.
    ///
    /// # Parameters
    ///
    /// - `fd_set`: Raw pointer to an fd_set structure
    /// - `fd`: The file descriptor number to check
    ///
    /// # Returns
    ///
    /// This function returns `Ok(true)` if the file descriptor is set, `Ok(false)` if it is not
    /// set, or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of range
    /// (`fd >= FD_SETSIZE`).
    ///
    /// # Safety
    ///
    /// The caller must ensure that `fd_set` points to a valid `fd_set` structure.
    ///
    pub unsafe fn is_set_raw(fd_set: *const fd_set, fd: usize) -> Result<bool, FdSetError> {
        // SAFETY: Caller must ensure that `fd_set` is a valid pointer.
        unsafe { (*fd_set).is_set(fd) }
    }

    ///
    /// # Description
    ///
    /// Sets a specific file descriptor bit in the fd_set.
    ///
    /// # Parameters
    ///
    /// - `fd`: The file descriptor number to set
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on success or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of
    /// range (`fd >= FD_SETSIZE`).
    pub fn set_bit(&mut self, fd: usize) -> Result<(), FdSetError> {
        if fd >= FD_SETSIZE {
            return Err(FdSetError::FileDescriptorOutOfRange);
        }
        let (word_index, mask): (usize, c_ulong) = Self::bit_position(fd);
        let base: *mut c_ulong = ::core::ptr::addr_of_mut!(self.fds_bits) as *mut c_ulong;
        unsafe {
            let word: *mut c_ulong = base.add(word_index);
            word.write(word.read() | mask);
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Raw pointer version of `set_bit()`.
    ///
    /// # Parameters
    ///
    /// - `fd_set`: Raw mutable pointer to an fd_set structure
    /// - `fd`: The file descriptor number to set
    ///
    /// # Safety
    ///
    /// The caller must ensure that `fd_set` points to a valid, mutable `fd_set` structure.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on success or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of
    /// range (`fd >= FD_SETSIZE`).
    pub unsafe fn set_bit_raw(fd_set: *mut fd_set, fd: usize) -> Result<(), FdSetError> {
        // SAFETY: Caller must ensure that `fd_set` is a valid pointer.
        unsafe { (*fd_set).set_bit(fd) }
    }

    ///
    /// # Description
    ///
    /// Clears a specific file descriptor bit in the fd_set.
    ///
    /// # Parameters
    ///
    /// - `fd`: The file descriptor number to clear
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on success or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of
    /// range (`fd >= FD_SETSIZE`).
    pub fn clear_bit(&mut self, fd: usize) -> Result<(), FdSetError> {
        if fd >= FD_SETSIZE {
            return Err(FdSetError::FileDescriptorOutOfRange);
        }
        let (word_index, mask): (usize, c_ulong) = Self::bit_position(fd);
        let base: *mut c_ulong = ::core::ptr::addr_of_mut!(self.fds_bits) as *mut c_ulong;
        unsafe {
            let word: *mut c_ulong = base.add(word_index);
            word.write(word.read() & !mask);
        }
        Ok(())
    }

    ///
    /// # Description
    ///
    /// Raw pointer version of `clear_bit()`.
    ///
    /// # Parameters
    ///
    /// - `fd_set`: Raw mutable pointer to an fd_set structure
    /// - `fd`: The file descriptor number to clear
    ///
    /// # Safety
    ///
    /// The caller must ensure that `fd_set` points to a valid, mutable `fd_set` structure.
    ///
    /// # Return Value
    ///
    /// Returns `Ok(())` on success or `Err(FdSetError::FileDescriptorOutOfRange)` if `fd` is out of
    /// range (`fd >= FD_SETSIZE`).
    pub unsafe fn clear_bit_raw(fd_set: *mut fd_set, fd: usize) -> Result<(), FdSetError> {
        // SAFETY: Caller must ensure that `fd_set` is a valid pointer.
        unsafe { (*fd_set).clear_bit(fd) }
    }

    ///
    /// # Description
    ///
    /// Clears all file descriptor bits in the fd_set.
    ///
    pub fn zero(&mut self) {
        // Avoid taking references to a packed field; operate through raw pointer.
        let base: *mut c_ulong = ::core::ptr::addr_of_mut!(self.fds_bits) as *mut c_ulong;
        for i in 0..FD_SET_WORD_COUNT {
            unsafe { base.add(i).write(0) };
        }
    }

    ///
    /// # Description
    ///
    /// Raw pointer version of `zero()`.
    ///
    /// # Parameters
    ///
    /// - `fd_set`: Raw mutable pointer to an fd_set structure to clear
    ///
    /// # Safety
    ///
    /// The caller must ensure that `fd_set` points to a valid, mutable `fd_set` structure.
    ///
    pub unsafe fn zero_raw(fd_set: *mut fd_set) {
        // SAFETY: Caller must ensure that `fd_set` is a valid pointer.
        unsafe {
            (*fd_set).zero();
        }
    }
}

impl Default for fd_set {
    fn default() -> Self {
        Self {
            fds_bits: [0; FD_SET_WORD_COUNT],
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timeval {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_usec: suseconds_t,
}

/// Errors that can occur when converting a `timeval` to a `timespec`.
#[derive(Debug, Clone, Copy)]
pub enum TimevalToTimespecParseError {
    /// Error code indicating failure to parse `tv_sec` field.
    FailedToParseTvSec,
    /// Error code indicating failure to parse `tv_usec` field.
    FailedToParseTvUsec,
}

impl TryFrom<timeval> for crate::time::timespec {
    type Error = TimevalToTimespecParseError;

    fn try_from(tv: timeval) -> Result<Self, Self::Error> {
        // Check if `tv_sec` is valid.
        if tv.tv_sec < 0 {
            return Err(TimevalToTimespecParseError::FailedToParseTvSec);
        }

        // Check if `tv_usec` is valid.
        if tv.tv_usec < 0 || tv.tv_usec >= MICROSECONDS_PER_SECOND {
            return Err(TimevalToTimespecParseError::FailedToParseTvUsec);
        }

        // Handle wrap around for nanoseconds.
        let mut sec: time_t = tv.tv_sec;
        let mut nsec: suseconds_t = tv.tv_usec * (NANOSECONDS_PER_SECOND / MICROSECONDS_PER_SECOND);
        if nsec >= NANOSECONDS_PER_SECOND {
            sec += 1;
            nsec -= NANOSECONDS_PER_SECOND;
        }

        Ok(crate::time::timespec {
            tv_sec: sec,
            tv_nsec: nsec,
        })
    }
}
