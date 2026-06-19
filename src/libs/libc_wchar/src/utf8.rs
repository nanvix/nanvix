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
mod test {
    use super::{
        decode,
        encode,
        Decoded,
    };

    /// Extracts the decoded code point and consumed length, or `None` for non-`Ok` outcomes.
    fn as_ok(d: Decoded) -> Option<(u32, usize)> {
        match d {
            Decoded::Ok(cp, len) => Some((cp, len)),
            _ => None,
        }
    }

    #[test]
    fn test_decode_ascii() {
        assert_eq!(as_ok(decode(&[0x41])), Some((0x41, 1)));
        assert_eq!(as_ok(decode(&[0x00])), Some((0x00, 1)));
    }

    #[test]
    fn test_decode_two_byte() {
        // U+00E9 (LATIN SMALL LETTER E WITH ACUTE) = 0xC3 0xA9.
        assert_eq!(as_ok(decode(&[0xC3, 0xA9])), Some((0xE9, 2)));
    }

    #[test]
    fn test_decode_three_byte() {
        // U+20AC (EURO SIGN) = 0xE2 0x82 0xAC.
        assert_eq!(as_ok(decode(&[0xE2, 0x82, 0xAC])), Some((0x20AC, 3)));
    }

    #[test]
    fn test_decode_four_byte() {
        // U+1F600 (GRINNING FACE) = 0xF0 0x9F 0x98 0x80.
        assert_eq!(as_ok(decode(&[0xF0, 0x9F, 0x98, 0x80])), Some((0x1F600, 4)));
    }

    #[test]
    fn test_decode_incomplete() {
        // A valid two-byte lead with no continuation byte available yet.
        assert!(matches!(decode(&[0xC3]), Decoded::Incomplete));
    }

    #[test]
    fn test_decode_invalid_lead() {
        // 0xFF is never a valid leading byte.
        assert!(matches!(decode(&[0xFF]), Decoded::Invalid));
    }

    #[test]
    fn test_decode_invalid_continuation() {
        // The second byte is not a continuation byte.
        assert!(matches!(decode(&[0xC3, 0x00]), Decoded::Invalid));
    }

    #[test]
    fn test_decode_rejects_overlong() {
        // 0xC0 0x80 is an overlong encoding of U+0000.
        assert!(matches!(decode(&[0xC0, 0x80]), Decoded::Invalid));
    }

    #[test]
    fn test_decode_rejects_surrogate() {
        // 0xED 0xA0 0x80 encodes the surrogate U+D800.
        assert!(matches!(decode(&[0xED, 0xA0, 0x80]), Decoded::Invalid));
    }

    #[test]
    fn test_encode_ascii() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0x41, &mut out), Some(1));
        assert_eq!(out[0], 0x41);
    }

    #[test]
    fn test_encode_two_byte() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0xE9, &mut out), Some(2));
        assert_eq!(&out[..2], &[0xC3, 0xA9]);
    }

    #[test]
    fn test_encode_three_byte() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0x20AC, &mut out), Some(3));
        assert_eq!(&out[..3], &[0xE2, 0x82, 0xAC]);
    }

    #[test]
    fn test_encode_four_byte() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0x1F600, &mut out), Some(4));
        assert_eq!(&out[..4], &[0xF0, 0x9F, 0x98, 0x80]);
    }

    #[test]
    fn test_encode_rejects_out_of_range() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0x11_0000, &mut out), None);
    }

    #[test]
    fn test_encode_rejects_surrogate() {
        let mut out: [u8; 4] = [0; 4];
        assert_eq!(encode(0xD800, &mut out), None);
    }

    #[test]
    fn test_round_trip() {
        for cp in [0x41u32, 0xE9, 0x20AC, 0x1F600] {
            let mut out: [u8; 4] = [0; 4];
            let len: usize = encode(cp, &mut out).unwrap_or_default();
            assert_eq!(as_ok(decode(&out[..len])), Some((cp, len)));
        }
    }
}
