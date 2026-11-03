// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
