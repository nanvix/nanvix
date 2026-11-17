// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

extern crate alloc;

use self::alloc::vec::Vec;
use super::{
    digit_value,
    Cursor,
    Length,
};
use ::core::ffi::VaList;
use ::sysapi::{
    errno::ENOMEM,
    ffi::{
        c_char,
        c_int,
        c_void,
    },
};

/// Collects only the floating input item, never a whole line or its delimiter.
struct Item<'a> {
    cur: &'a mut Cursor,
    bytes: Vec<u8>,
    remaining: usize,
}

impl Item<'_> {
    unsafe fn peek(&mut self) -> Option<u8> {
        if self.remaining == 0 {
            None
        } else {
            unsafe { self.cur.peek() }
        }
    }

    unsafe fn take_if(&mut self, accepts: impl Fn(u8) -> bool) -> Result<bool, c_int> {
        let Some(c) = (unsafe { self.peek() }).filter(|&c| accepts(c)) else {
            return Ok(false);
        };
        self.bytes.try_reserve(1).map_err(|_| ENOMEM)?;
        self.bytes.push(c);
        unsafe { self.cur.bump() };
        self.remaining -= 1;
        Ok(true)
    }

    unsafe fn keyword(&mut self, word: &[u8]) -> Result<bool, c_int> {
        for &expected in word {
            if !unsafe { self.take_if(|c| c.to_ascii_lowercase() == expected) }? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    unsafe fn digits(&mut self, base: u32) -> Result<usize, c_int> {
        let mut count: usize = 0;
        while unsafe { self.take_if(|c| digit_value(c, base).is_some()) }? {
            count += 1;
        }
        Ok(count)
    }

    /// Consumes the longest prefix of a floating subject sequence, even if incomplete.
    unsafe fn collect(&mut self) -> Result<bool, c_int> {
        unsafe { self.take_if(|c| matches!(c, b'+' | b'-')) }?;
        match unsafe { self.peek() }.map(|c| c.to_ascii_lowercase()) {
            Some(b'i') => {
                if !unsafe { self.keyword(b"inf") }? {
                    return Ok(false);
                }
                return if unsafe { self.peek() }.is_some_and(|c| c.eq_ignore_ascii_case(&b'i')) {
                    unsafe { self.keyword(b"inity") }
                } else {
                    Ok(true)
                };
            },
            Some(b'n') => {
                if !unsafe { self.keyword(b"nan") }? {
                    return Ok(false);
                }
                if unsafe { self.take_if(|c| c == b'(') }? {
                    while unsafe { self.take_if(|c| c.is_ascii_alphanumeric() || c == b'_') }? {}
                    return unsafe { self.take_if(|c| c == b')') };
                }
                return Ok(true);
            },
            _ => {},
        }

        let zero: bool = unsafe { self.take_if(|c| c == b'0') }?;
        let hex: bool = zero && unsafe { self.take_if(|c| matches!(c, b'x' | b'X')) }?;
        let base: u32 = if hex { 16 } else { 10 };
        let mut digits: usize = usize::from(zero && !hex) + unsafe { self.digits(base) }?;
        if unsafe { self.take_if(|c| c == b'.') }? {
            digits += unsafe { self.digits(base) }?;
        }
        if digits == 0 {
            return Ok(false);
        }
        let exponent: u8 = if hex { b'p' } else { b'e' };
        if unsafe { self.take_if(|c| c.to_ascii_lowercase() == exponent) }? {
            unsafe { self.take_if(|c| matches!(c, b'+' | b'-')) }?;
            if unsafe { self.digits(10) }? == 0 {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Converts a bounded input item with the existing C numeric parser.
pub(super) unsafe fn convert(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    width: usize,
    length: Length,
    suppress: bool,
) -> Result<bool, c_int> {
    extern "C" {
        fn strtod(s: *const c_char, end: *mut *mut c_char) -> f64;
    }

    let mut item: Item<'_> = Item {
        cur,
        bytes: Vec::new(),
        remaining: if width == 0 { usize::MAX } else { width },
    };
    if !unsafe { item.collect() }? {
        return Ok(false);
    }
    if suppress {
        return Ok(true);
    }
    item.bytes.try_reserve(1).map_err(|_| ENOMEM)?;
    item.bytes.push(0);
    let input: *const c_char = item.bytes.as_ptr().cast::<c_char>();
    let mut end: *mut c_char = core::ptr::null_mut();
    let value: f64 = unsafe { strtod(input, &raw mut end) };
    if end != unsafe { input.add(item.bytes.len() - 1) }.cast_mut() {
        return Ok(false);
    }
    // SAFETY: the caller provides storage matching the format's length modifier.
    let dst: *mut c_void = unsafe { ap.next_arg::<*mut c_void>() };
    match length {
        Length::Long => unsafe { *dst.cast::<f64>() = value },
        Length::LongLong => unsafe { store_long_double(dst, value) },
        _ => unsafe { *dst.cast::<f32>() = value as f32 },
    }
    Ok(true)
}

/// Stores double-precision results using the C long double representation, as strtold does.
unsafe fn store_long_double(dst: *mut c_void, value: f64) {
    #[cfg(target_os = "windows")]
    unsafe {
        *dst.cast::<f64>() = value;
    }
    #[cfg(not(target_os = "windows"))]
    unsafe {
        core::arch::asm!(
            "fld qword ptr [{value}]",
            "fstp tbyte ptr [{dst}]",
            value = in(reg) &value,
            dst = in(reg) dst,
            options(nostack),
        );
    }
}
