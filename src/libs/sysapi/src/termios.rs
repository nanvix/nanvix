// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! General terminal interface.
//!
//! Declares the [`Termios`] structure and the flag and control-character constants used by the
//! terminal-attribute interfaces (`tcgetattr`/`tcsetattr` and the `TCGETS`/`TCSETS` ioctls). The
//! layout mirrors `struct termios` in `include/termios.h` exactly so that the C ABI structure can
//! be exchanged byte-for-byte with the vfsd console backend.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Types
//==================================================================================================

/// Terminal mode flags type (`c_iflag`/`c_oflag`/`c_cflag`/`c_lflag`).
pub type tcflag_t = u32;

/// Terminal control-character type (`c_cc` elements).
pub type cc_t = u8;

/// Terminal baud-rate type (`c_ispeed`/`c_ospeed`).
pub type speed_t = u32;

/// Number of control characters in [`Termios::c_cc`].
pub const NCCS: usize = 32;

//==================================================================================================
// Input flags (c_iflag)
//==================================================================================================

/// Map CR to NL on input.
pub const ICRNL: tcflag_t = 0x0100;
/// Enable start/stop output control.
pub const IXON: tcflag_t = 0x0400;

//==================================================================================================
// Output flags (c_oflag)
//==================================================================================================

/// Post-process output.
pub const OPOST: tcflag_t = 0x0001;
/// Map NL to CR-NL on output.
pub const ONLCR: tcflag_t = 0x0004;

//==================================================================================================
// Control flags (c_cflag)
//==================================================================================================

/// Character size mask: 8 bits.
pub const CS8: tcflag_t = 0x0030;
/// Enable receiver.
pub const CREAD: tcflag_t = 0x0080;

//==================================================================================================
// Local flags (c_lflag)
//==================================================================================================

/// Enable signals (`INTR`, `QUIT`, `SUSP`).
pub const ISIG: tcflag_t = 0x0001;
/// Canonical input (erase and kill processing).
pub const ICANON: tcflag_t = 0x0002;
/// Enable echo.
pub const ECHO: tcflag_t = 0x0008;
/// Echo erase character as error-correcting backspace.
pub const ECHOE: tcflag_t = 0x0010;
/// Echo `KILL`.
pub const ECHOK: tcflag_t = 0x0020;
/// Enable implementation-defined input processing.
pub const IEXTEN: tcflag_t = 0x8000;

//==================================================================================================
// Control-character indices (c_cc)
//==================================================================================================

/// `INTR` character index.
pub const VINTR: usize = 0;
/// `QUIT` character index.
pub const VQUIT: usize = 1;
/// `ERASE` character index.
pub const VERASE: usize = 2;
/// `KILL` character index.
pub const VKILL: usize = 3;
/// `EOF` character index.
pub const VEOF: usize = 4;
/// `TIME` value index (non-canonical read timeout).
pub const VTIME: usize = 5;
/// `MIN` value index (non-canonical read minimum).
pub const VMIN: usize = 6;
/// `START` character index.
pub const VSTART: usize = 8;
/// `STOP` character index.
pub const VSTOP: usize = 9;
/// `SUSP` character index.
pub const VSUSP: usize = 10;
/// `EOL` character index.
pub const VEOL: usize = 11;
/// `REPRINT` character index.
pub const VREPRINT: usize = 12;
/// `DISCARD` character index.
pub const VDISCARD: usize = 13;
/// `WERASE` character index.
pub const VWERASE: usize = 14;
/// `LNEXT` character index.
pub const VLNEXT: usize = 15;
/// `EOL2` character index.
pub const VEOL2: usize = 16;

//==================================================================================================
// Baud rates
//==================================================================================================

/// Hang up (zero baud); lowest baud in the contiguous low range.
pub const B0: speed_t = 0x0000;
/// 38400 baud (the default console line speed); highest baud in the contiguous low range.
pub const B38400: speed_t = 0x000f;
/// Extended baud-rate flag, set by every speed above `B38400`; not itself a valid baud rate.
pub const CBAUDEX: speed_t = 0x1000;
/// 57600 baud; lowest baud in the extended range.
pub const B57600: speed_t = 0x1001;
/// 4000000 baud; highest baud in the extended range.
pub const B4000000: speed_t = 0x100f;

//==================================================================================================
// Optional actions for tcsetattr()
//==================================================================================================

/// Apply the change immediately.
pub const TCSANOW: i32 = 0;
/// Apply the change after queued output is transmitted.
pub const TCSADRAIN: i32 = 1;
/// Apply the change after queued output is transmitted and pending input is discarded.
pub const TCSAFLUSH: i32 = 2;

//==================================================================================================
// Queue selectors for tcflush()
//==================================================================================================

/// Flush pending input.
pub const TCIFLUSH: i32 = 0;
/// Flush untransmitted output.
pub const TCOFLUSH: i32 = 1;
/// Flush both pending input and untransmitted output.
pub const TCIOFLUSH: i32 = 2;

//==================================================================================================
// Actions for tcflow()
//==================================================================================================

/// Suspend output.
pub const TCOOFF: i32 = 0;
/// Resume suspended output.
pub const TCOON: i32 = 1;
/// Transmit a STOP character to suspend input.
pub const TCIOFF: i32 = 2;
/// Transmit a START character to resume input.
pub const TCION: i32 = 3;

//==================================================================================================
// Structures
//==================================================================================================

/// Terminal attributes.
///
/// The field order and types match `struct termios` in `include/termios.h`, so a value of this type
/// may be exchanged byte-for-byte with a C caller and with the vfsd console backend.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Termios {
    /// Input modes.
    pub c_iflag: tcflag_t,
    /// Output modes.
    pub c_oflag: tcflag_t,
    /// Control modes.
    pub c_cflag: tcflag_t,
    /// Local modes.
    pub c_lflag: tcflag_t,
    /// Line discipline.
    pub c_line: cc_t,
    /// Control characters.
    pub c_cc: [cc_t; NCCS],
    /// Input speed.
    pub c_ispeed: speed_t,
    /// Output speed.
    pub c_ospeed: speed_t,
}

impl Termios {
    /// Size of the terminal-attributes structure, in bytes.
    pub const SIZE: usize = ::core::mem::size_of::<Self>();
    const C_IFLAG_OFFSET: usize = 0;
    const C_OFLAG_OFFSET: usize = Self::C_IFLAG_OFFSET + ::core::mem::size_of::<tcflag_t>();
    const C_CFLAG_OFFSET: usize = Self::C_OFLAG_OFFSET + ::core::mem::size_of::<tcflag_t>();
    const C_LFLAG_OFFSET: usize = Self::C_CFLAG_OFFSET + ::core::mem::size_of::<tcflag_t>();
    const C_LINE_OFFSET: usize = Self::C_LFLAG_OFFSET + ::core::mem::size_of::<tcflag_t>();
    const C_CC_OFFSET: usize = Self::C_LINE_OFFSET + ::core::mem::size_of::<cc_t>();
    const C_ISPEED_OFFSET: usize =
        Self::align_up(Self::C_CC_OFFSET + NCCS, ::core::mem::align_of::<speed_t>());
    const C_ISPEED_END: usize = Self::C_ISPEED_OFFSET + ::core::mem::size_of::<speed_t>();
    const C_OSPEED_OFFSET: usize = Self::C_ISPEED_END;
    const C_OSPEED_END: usize = Self::C_OSPEED_OFFSET + ::core::mem::size_of::<speed_t>();

    const fn align_up(value: usize, align: usize) -> usize {
        (value + align - 1) & !(align - 1)
    }

    fn read_u32(raw: &[u8; Self::SIZE], start: usize) -> u32 {
        let mut bytes: [u8; ::core::mem::size_of::<u32>()] = [0; ::core::mem::size_of::<u32>()];
        bytes.copy_from_slice(&raw[start..start + ::core::mem::size_of::<u32>()]);
        u32::from_ne_bytes(bytes)
    }

    /// Returns the default attributes for an interactive console.
    ///
    /// The defaults enable canonical line editing with echo (`ICANON | ECHO`) and seed the standard
    /// control characters, matching a freshly opened Linux terminal closely enough that
    /// terminal-aware programs probing the console see sane values.
    pub const fn console_default() -> Self {
        let mut c_cc: [cc_t; NCCS] = [0; NCCS];
        c_cc[VINTR] = 3; // ^C
        c_cc[VQUIT] = 28; // ^backslash
        c_cc[VERASE] = 127; // DEL
        c_cc[VKILL] = 21; // ^U
        c_cc[VEOF] = 4; // ^D
        c_cc[VTIME] = 0;
        c_cc[VMIN] = 1;
        c_cc[VSTART] = 17; // ^Q
        c_cc[VSTOP] = 19; // ^S
        c_cc[VSUSP] = 26; // ^Z
        c_cc[VEOL] = 0;
        c_cc[VREPRINT] = 18; // ^R
        c_cc[VDISCARD] = 15; // ^O
        c_cc[VWERASE] = 23; // ^W
        c_cc[VLNEXT] = 22; // ^V
        c_cc[VEOL2] = 0;
        Self {
            c_iflag: ICRNL | IXON,
            c_oflag: OPOST | ONLCR,
            c_cflag: CREAD | CS8,
            c_lflag: ISIG | ICANON | ECHO | ECHOE | ECHOK | IEXTEN,
            c_line: 0,
            c_cc,
            c_ispeed: B38400,
            c_ospeed: B38400,
        }
    }

    /// Serializes the structure to its C ABI byte representation.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes: [u8; Self::SIZE] = [0; Self::SIZE];
        bytes[Self::C_IFLAG_OFFSET..Self::C_OFLAG_OFFSET]
            .copy_from_slice(&self.c_iflag.to_ne_bytes());
        bytes[Self::C_OFLAG_OFFSET..Self::C_CFLAG_OFFSET]
            .copy_from_slice(&self.c_oflag.to_ne_bytes());
        bytes[Self::C_CFLAG_OFFSET..Self::C_LFLAG_OFFSET]
            .copy_from_slice(&self.c_cflag.to_ne_bytes());
        bytes[Self::C_LFLAG_OFFSET..Self::C_LINE_OFFSET]
            .copy_from_slice(&self.c_lflag.to_ne_bytes());
        bytes[Self::C_LINE_OFFSET] = self.c_line;
        bytes[Self::C_CC_OFFSET..Self::C_CC_OFFSET + NCCS].copy_from_slice(&self.c_cc);
        bytes[Self::C_ISPEED_OFFSET..Self::C_ISPEED_END]
            .copy_from_slice(&self.c_ispeed.to_ne_bytes());
        bytes[Self::C_OSPEED_OFFSET..Self::C_OSPEED_END]
            .copy_from_slice(&self.c_ospeed.to_ne_bytes());
        bytes
    }

    /// Reconstructs a value from its byte representation.
    ///
    /// Bytes beyond [`Termios::SIZE`] are ignored; missing bytes are read as zero. The bytes are
    /// read without assuming alignment.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut raw: [u8; Self::SIZE] = [0; Self::SIZE];
        let n: usize = if bytes.len() < Self::SIZE {
            bytes.len()
        } else {
            Self::SIZE
        };
        raw[..n].copy_from_slice(&bytes[..n]);
        Self {
            c_iflag: Self::read_u32(&raw, Self::C_IFLAG_OFFSET),
            c_oflag: Self::read_u32(&raw, Self::C_OFLAG_OFFSET),
            c_cflag: Self::read_u32(&raw, Self::C_CFLAG_OFFSET),
            c_lflag: Self::read_u32(&raw, Self::C_LFLAG_OFFSET),
            c_line: raw[Self::C_LINE_OFFSET],
            c_cc: {
                let mut c_cc: [cc_t; NCCS] = [0; NCCS];
                c_cc.copy_from_slice(&raw[Self::C_CC_OFFSET..Self::C_CC_OFFSET + NCCS]);
                c_cc
            },
            c_ispeed: Self::read_u32(&raw, Self::C_ISPEED_OFFSET),
            c_ospeed: Self::read_u32(&raw, Self::C_OSPEED_OFFSET),
        }
    }
}
