// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Device control operations.
//!
//! Declares the [`Winsize`] structure and the terminal-control request codes used by `ioctl()`. The
//! layout and values mirror `include/sys/ioctl.h` and the Linux-compatible request numbers so the
//! C ABI structure can be exchanged byte-for-byte with the vfsd console backend.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Requests
//==================================================================================================

/// Get the terminal attributes (`struct termios`).
pub const TCGETS: i32 = 0x5401;
/// Set the terminal attributes (`struct termios`).
pub const TCSETS: i32 = 0x5402;
/// Get the foreground process group of the terminal (`pid_t`).
pub const TIOCGPGRP: i32 = 0x540f;
/// Set the foreground process group of the terminal (`pid_t`).
pub const TIOCSPGRP: i32 = 0x5410;
/// Get the terminal window size (`struct winsize`).
pub const TIOCGWINSZ: i32 = 0x5413;
/// Set the terminal window size (`struct winsize`).
pub const TIOCSWINSZ: i32 = 0x5414;
/// Get the number of bytes available to read.
pub const FIONREAD: i32 = 0x541b;

//==================================================================================================
// Request Encoding
//==================================================================================================

/// Number of bits used for an ioctl request number.
pub const _IOC_NRBITS: u32 = 8;
/// Number of bits used for an ioctl request type.
pub const _IOC_TYPEBITS: u32 = 8;
/// Number of bits used for an ioctl request payload size.
pub const _IOC_SIZEBITS: u32 = 14;
/// Number of bits used for an ioctl transfer direction.
pub const _IOC_DIRBITS: u32 = 2;

/// Bit offset of the ioctl request number.
pub const _IOC_NRSHIFT: u32 = 0;
/// Bit offset of the ioctl request type.
pub const _IOC_TYPESHIFT: u32 = 8;
/// Bit offset of the ioctl request payload size.
pub const _IOC_SIZESHIFT: u32 = 16;
/// Bit offset of the ioctl transfer direction.
pub const _IOC_DIRSHIFT: u32 = 30;

/// Indicates an ioctl request with no data transfer.
pub const _IOC_NONE: u32 = 0;
/// Indicates an ioctl request that writes data to the device.
pub const _IOC_WRITE: u32 = 1;
/// Indicates an ioctl request that reads data from the device.
pub const _IOC_READ: u32 = 2;

//==================================================================================================
// Structures
//==================================================================================================

/// Terminal window size.
///
/// The field order and types match `struct winsize` in `include/sys/ioctl.h`, so a value of this
/// type may be exchanged byte-for-byte with a C caller and with the vfsd console backend.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Winsize {
    /// Rows, in characters.
    pub ws_row: u16,
    /// Columns, in characters.
    pub ws_col: u16,
    /// Horizontal size, in pixels.
    pub ws_xpixel: u16,
    /// Vertical size, in pixels.
    pub ws_ypixel: u16,
}

impl Winsize {
    /// Size of the window-size structure, in bytes.
    pub const SIZE: usize = ::core::mem::size_of::<Self>();

    /// Returns the default window size for an interactive console (24 rows by 80 columns).
    pub const fn console_default() -> Self {
        Self {
            ws_row: 24,
            ws_col: 80,
            ws_xpixel: 0,
            ws_ypixel: 0,
        }
    }

    /// Borrows the structure as a byte slice for byte-for-byte transfer.
    pub fn as_bytes(&self) -> &[u8] {
        // Safety: `Winsize` is `repr(C)` plain-old-data; exposing its representation as bytes is
        // sound and the slice borrows `self`.
        unsafe { ::core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Reconstructs a value from its byte representation.
    ///
    /// Bytes beyond [`Winsize::SIZE`] are ignored; missing bytes are read as zero. The bytes are
    /// read without assuming alignment.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut raw: [u8; Self::SIZE] = [0; Self::SIZE];
        let n: usize = if bytes.len() < Self::SIZE {
            bytes.len()
        } else {
            Self::SIZE
        };
        raw[..n].copy_from_slice(&bytes[..n]);
        // Safety: `raw` is exactly `SIZE` bytes and every bit pattern is a valid `Winsize`, so the
        // unaligned read yields a well-defined value.
        unsafe { ::core::ptr::read_unaligned(raw.as_ptr() as *const Self) }
    }
}
