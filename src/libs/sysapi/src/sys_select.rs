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
const MICROSECONDS_PER_SECOND: suseconds_t = 1_000_000;

/// Nanoseconds in a second.
const NANOSECONDS_PER_SECOND: suseconds_t = 1_000_000_000;

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
#[repr(C)]
pub struct fd_set {
    /// Bit fields for tracked file descriptors.
    pub fds_bits: [c_ulong; FD_SET_WORD_COUNT],
}
::static_assert::assert_eq_size!(fd_set, size_of::<[c_ulong; FD_SET_WORD_COUNT]>());
::static_assert::assert_eq_align!(fd_set, core::mem::align_of::<c_ulong>());

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

    /// Fixed-width size of the wire encoding produced by [`fd_set::to_bytes()`].
    ///
    /// This is independent of the width of [`c_ulong`] (4 bytes on x86 ILP32 guests, 8 bytes on
    /// x86_64 LP64 hosts), so the on-the-wire representation is identical across targets.
    pub const WIRE_SIZE: usize = FD_SET_WORD_COUNT * size_of::<c_ulong>();

    ///
    /// # Description
    ///
    /// Serializes the file descriptor set into its fixed-width wire representation.
    ///
    /// # Return Value
    ///
    /// Returns a byte array holding the wire encoding of the file descriptor set.
    ///
    pub fn to_bytes(&self) -> [u8; Self::WIRE_SIZE] {
        let mut bytes: [u8; Self::WIRE_SIZE] = [0; Self::WIRE_SIZE];
        // Operate through a raw pointer to avoid creating an unaligned reference to a packed field.
        let base: *const c_ulong = ::core::ptr::addr_of!(self.fds_bits) as *const c_ulong;
        for index in 0..FD_SET_WORD_COUNT {
            // SAFETY: `index` is within the bounds of `fds_bits`.
            let word: c_ulong = unsafe { base.add(index).read_unaligned() };
            let offset: usize = index * size_of::<c_ulong>();
            bytes[offset..offset + size_of::<c_ulong>()].copy_from_slice(&word.to_ne_bytes());
        }
        bytes
    }

    ///
    /// # Description
    ///
    /// Deserializes a file descriptor set from the fixed-width wire representation produced by
    /// [`fd_set::to_bytes()`].
    ///
    /// # Parameters
    ///
    /// - `bytes`: Wire encoding of the file descriptor set.
    ///
    /// # Return Value
    ///
    /// Returns the reconstructed file descriptor set.
    ///
    pub fn from_bytes(bytes: [u8; Self::WIRE_SIZE]) -> Self {
        let mut fds_bits: [c_ulong; FD_SET_WORD_COUNT] = [0; FD_SET_WORD_COUNT];
        for (index, word) in fds_bits.iter_mut().enumerate() {
            let offset: usize = index * size_of::<c_ulong>();
            let mut buffer: [u8; size_of::<c_ulong>()] = [0; size_of::<c_ulong>()];
            buffer.copy_from_slice(&bytes[offset..offset + size_of::<c_ulong>()]);
            *word = c_ulong::from_ne_bytes(buffer);
        }
        Self { fds_bits }
    }
}

impl Default for fd_set {
    fn default() -> Self {
        Self {
            fds_bits: [0; FD_SET_WORD_COUNT],
        }
    }
}

/// Time interval expressed in seconds and microseconds.
#[derive(Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct timeval {
    /// Seconds.
    pub tv_sec: time_t,
    /// Microseconds.
    pub tv_usec: suseconds_t,
}

/// Errors that can occur when serializing or deserializing a [`timeval`] wire payload.
#[derive(Debug, Clone, Copy)]
pub enum TimevalError {
    /// Error code indicating an invalid array size.
    InvalidArraySize,
    /// Error code indicating failure to parse `tv_sec` field.
    FailedToParseTvSec,
    /// Error code indicating failure to parse `tv_usec` field.
    FailedToParseTvUsec,
}

impl timeval {
    /// Size of the seconds field.
    const SIZE_OF_TV_SEC: usize = size_of::<time_t>();
    /// Offset of the seconds field.
    const OFFSET_OF_TV_SEC: usize = 0;
    /// Offset of the micro-seconds field.
    const OFFSET_OF_TV_USEC: usize = Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC;

    /// Wire format always uses i64 (8 bytes) for `tv_usec`, ensuring IPC compatibility
    /// between x86 guests (`c_long=i32`) and x86_64 hosts (`c_long=i64`).
    const WIRE_SIZE_OF_TV_USEC: usize = size_of::<i64>();
    /// Wire format size for IPC serialization (always 16 bytes).
    pub const WIRE_SIZE: usize = Self::SIZE_OF_TV_SEC + Self::WIRE_SIZE_OF_TV_USEC;

    /// Converts a `timeval` structure to a wire-format byte array.
    pub fn to_bytes(&self) -> [u8; Self::WIRE_SIZE] {
        let mut bytes: [u8; Self::WIRE_SIZE] = [0; Self::WIRE_SIZE];

        // Convert seconds field.
        bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
            .copy_from_slice(&self.tv_sec.to_ne_bytes());

        // Convert micro-seconds field (always as i64 for IPC compatibility).
        // Cast is needed on x86 (c_long=i32→i64) but is a no-op on x86_64 (c_long=i64).
        #[cfg_attr(target_arch = "x86_64", allow(clippy::unnecessary_cast))]
        let usec_bytes = (self.tv_usec as i64).to_ne_bytes();
        bytes[Self::OFFSET_OF_TV_USEC..Self::OFFSET_OF_TV_USEC + Self::WIRE_SIZE_OF_TV_USEC]
            .copy_from_slice(&usec_bytes);

        bytes
    }

    /// Tries to convert a `timeval` structure from a wire-format byte array.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, TimevalError> {
        // Check if the array has the correct wire size.
        if bytes.len() != Self::WIRE_SIZE {
            return Err(TimevalError::InvalidArraySize);
        }

        // Parse seconds field.
        let tv_sec: time_t = time_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
                .try_into()
                .map_err(|_| TimevalError::FailedToParseTvSec)?,
        );

        // Parse micro-seconds field (always read as i64, then convert to suseconds_t).
        // The checked conversion rejects out-of-range values instead of silently wrapping on
        // 32-bit targets where suseconds_t (c_long) is narrower than the i64 wire value.
        let raw_tv_usec: i64 = i64::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_USEC..Self::OFFSET_OF_TV_USEC + Self::WIRE_SIZE_OF_TV_USEC]
                .try_into()
                .map_err(|_| TimevalError::FailedToParseTvUsec)?,
        );
        #[cfg_attr(
            target_arch = "x86_64",
            allow(clippy::unnecessary_fallible_conversions)
        )]
        let tv_usec: suseconds_t =
            suseconds_t::try_from(raw_tv_usec).map_err(|_| TimevalError::FailedToParseTvUsec)?;

        Ok(Self { tv_sec, tv_usec })
    }
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
