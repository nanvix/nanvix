// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Enumerations
//==================================================================================================

/// Outcome of decoding a single UTF-8 multibyte sequence.
pub enum Decoded {
    /// A complete code point was decoded, consuming the given number of bytes.
    Ok(u32, usize),
    /// The available bytes are a valid prefix of a longer sequence.
    Incomplete,
    /// The bytes do not form a valid UTF-8 sequence.
    Invalid,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns the total length of the UTF-8 sequence introduced by the leading byte `b`, or `None`
/// if `b` is not a valid leading byte.
fn seq_len(b: u8) -> Option<usize> {
    if b & 0x80 == 0x00 {
        Some(1)
    } else if b & 0xe0 == 0xc0 {
        Some(2)
    } else if b & 0xf0 == 0xe0 {
        Some(3)
    } else if b & 0xf8 == 0xf0 {
        Some(4)
    } else {
        None
    }
}

/// Decodes a single UTF-8 sequence from `bytes`.
pub fn decode(bytes: &[u8]) -> Decoded {
    let first: u8 = match bytes.first() {
        Some(&b) => b,
        None => return Decoded::Incomplete,
    };

    let len: usize = match seq_len(first) {
        Some(l) => l,
        None => return Decoded::Invalid,
    };

    if len == 1 {
        return Decoded::Ok(u32::from(first), 1);
    }

    if bytes.len() < len {
        // Validate the available continuation bytes before declaring the prefix incomplete.
        for &c in &bytes[1..] {
            if c & 0xc0 != 0x80 {
                return Decoded::Invalid;
            }
        }
        return Decoded::Incomplete;
    }

    let mut cp: u32 = match len {
        2 => u32::from(first & 0x1f),
        3 => u32::from(first & 0x0f),
        _ => u32::from(first & 0x07),
    };
    for &c in &bytes[1..len] {
        if c & 0xc0 != 0x80 {
            return Decoded::Invalid;
        }
        cp = (cp << 6) | u32::from(c & 0x3f);
    }

    // Reject overlong encodings, surrogates, and out-of-range code points.
    let min: u32 = match len {
        2 => 0x80,
        3 => 0x800,
        _ => 0x10000,
    };
    if cp < min || cp > 0x10_ffff || (0xd800..=0xdfff).contains(&cp) {
        return Decoded::Invalid;
    }

    Decoded::Ok(cp, len)
}

/// Encodes the code point `cp` as UTF-8 into `out`, returning the number of bytes written, or
/// `None` if `cp` is not a valid Unicode scalar value.
pub fn encode(cp: u32, out: &mut [u8; 4]) -> Option<usize> {
    if cp > 0x10_ffff || (0xd800..=0xdfff).contains(&cp) {
        return None;
    }
    if cp < 0x80 {
        out[0] = u8::try_from(cp).unwrap_or(0);
        Some(1)
    } else if cp < 0x800 {
        out[0] = u8::try_from(0xc0 | (cp >> 6)).unwrap_or(0);
        out[1] = u8::try_from(0x80 | (cp & 0x3f)).unwrap_or(0);
        Some(2)
    } else if cp < 0x10000 {
        out[0] = u8::try_from(0xe0 | (cp >> 12)).unwrap_or(0);
        out[1] = u8::try_from(0x80 | ((cp >> 6) & 0x3f)).unwrap_or(0);
        out[2] = u8::try_from(0x80 | (cp & 0x3f)).unwrap_or(0);
        Some(3)
    } else {
        out[0] = u8::try_from(0xf0 | (cp >> 18)).unwrap_or(0);
        out[1] = u8::try_from(0x80 | ((cp >> 12) & 0x3f)).unwrap_or(0);
        out[2] = u8::try_from(0x80 | ((cp >> 6) & 0x3f)).unwrap_or(0);
        out[3] = u8::try_from(0x80 | (cp & 0x3f)).unwrap_or(0);
        Some(4)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
