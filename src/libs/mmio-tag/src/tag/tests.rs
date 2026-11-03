// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_new_from_bytes() {
    let bytes: [u8; TAG_LENGTH] = *b"TESTNAME";
    let tag: MmioTag = MmioTag::new(bytes);

    // Verify the tag was created successfully by checking Debug output.
    let debug_str = format!("{:?}", tag);
    assert_eq!(debug_str, "\"TESTNAME\"");
}

#[test]
fn test_from_u64_hex_zero() {
    let tag: MmioTag = MmioTag::from_u64_hex(0);
    let display_str = format!("{}", tag);
    assert_eq!(display_str, "00000000");
}

#[test]
fn test_from_u64_hex_nonzero() {
    let tag: MmioTag = MmioTag::from_u64_hex(0xDEADBEEF);
    let display_str = format!("{}", tag);
    assert_eq!(display_str, "DEADBEEF");
}

#[test]
fn test_from_u64_ascii() {
    // "RAMFS   " as big-endian bytes.
    let value: u64 = u64::from_be_bytes(*b"RAMFS   ");
    let tag: MmioTag = MmioTag::from_u64(value);
    let display_str = format!("{}", tag);
    assert_eq!(display_str, "RAMFS   ");
}

#[test]
fn test_from_u64_non_printable() {
    // Contains non-printable bytes.
    let value: u64 = 0x0102030405060708;
    let tag: MmioTag = MmioTag::from_u64(value);
    let display_str = format!("{}", tag);

    // Should be hex-encoded since input is non-printable.
    // Note: from_u64_hex encodes the lower 32 bits (8 nibbles) into TAG_LENGTH characters.
    assert_eq!(display_str, "05060708");
}

#[test]
fn test_from_name_short() {
    let tag: MmioTag = MmioTag::from_name("test");
    let display_str = format!("{}", tag);

    // Should be uppercased and padded with spaces.
    assert_eq!(display_str, "TEST    ");
}

#[test]
fn test_from_name_long() {
    let tag: MmioTag = MmioTag::from_name("verylongname");
    let display_str = format!("{}", tag);

    // Should be truncated to TAG_LENGTH.
    assert_eq!(display_str, "VERYLONG");
}

#[test]
fn test_from_name_uppercase() {
    let tag: MmioTag = MmioTag::from_name("IOAPIC");
    let display_str = format!("{}", tag);
    assert_eq!(display_str, "IOAPIC  ");
}

#[test]
fn test_equality() {
    let tag1: MmioTag = MmioTag::new(*b"TESTNAME");
    let tag2: MmioTag = MmioTag::new(*b"TESTNAME");
    let tag3: MmioTag = MmioTag::new(*b"DIFFNAME");

    assert_eq!(tag1, tag2);
    assert_ne!(tag1, tag3);
}

#[test]
fn test_ordering() {
    let tag_a: MmioTag = MmioTag::new(*b"AAAAAAAA");
    let tag_b: MmioTag = MmioTag::new(*b"BBBBBBBB");
    let tag_z: MmioTag = MmioTag::new(*b"ZZZZZZZZ");

    assert!(tag_a < tag_b);
    assert!(tag_b < tag_z);
    assert!(tag_a < tag_z);
}
