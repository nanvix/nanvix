// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::{
    MmioTag,
    TAG_LENGTH,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Tests if [`MmioTag::new()`] creates a tag from a raw byte array.
fn test_new_from_bytes() -> bool {
    let bytes: [u8; TAG_LENGTH] = *b"TESTNAME";
    let tag: MmioTag = MmioTag::new(bytes);

    // Verify the tag was created successfully by checking Debug output.
    let debug_str: alloc::string::String = alloc::format!("{:?}", tag);
    if debug_str != "\"TESTNAME\"" {
        error!("unexpected debug output: {}", debug_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_u64_hex()`] correctly encodes a value as hex.
fn test_from_u64_hex_zero() -> bool {
    let tag: MmioTag = MmioTag::from_u64_hex(0);
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    if display_str != "00000000" {
        error!("unexpected display for zero: {}", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_u64_hex()`] correctly encodes a non-zero value.
fn test_from_u64_hex_nonzero() -> bool {
    let tag: MmioTag = MmioTag::from_u64_hex(0xDEADBEEF);
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    if display_str != "DEADBEEF" {
        error!("unexpected display for 0xDEADBEEF: {}", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_u64()`] decodes printable ASCII bytes.
fn test_from_u64_ascii() -> bool {
    // "RAMFS   " as big-endian bytes.
    let value: u64 = u64::from_be_bytes(*b"RAMFS   ");
    let tag: MmioTag = MmioTag::from_u64(value);
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    if display_str != "RAMFS   " {
        error!("unexpected display for ASCII input: '{}'", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_u64()`] falls back to hex for non-printable input.
fn test_from_u64_non_printable() -> bool {
    // Contains non-printable bytes.
    let value: u64 = 0x0102030405060708;
    let tag: MmioTag = MmioTag::from_u64(value);
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    // Should be hex-encoded since input is non-printable.
    // Note: from_u64_hex encodes the lower 32 bits (8 nibbles) into TAG_LENGTH characters.
    if display_str != "05060708" {
        error!("unexpected display for non-printable input: '{}'", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_name()`] correctly derives a tag from a name.
fn test_from_name_short() -> bool {
    let tag: MmioTag = MmioTag::from_name("test");
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    // Should be uppercased and padded with spaces.
    if display_str != "TEST    " {
        error!("unexpected display for short name: '{}'", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_name()`] truncates long names.
fn test_from_name_long() -> bool {
    let tag: MmioTag = MmioTag::from_name("verylongname");
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    // Should be truncated to TAG_LENGTH.
    if display_str != "VERYLONG" {
        error!("unexpected display for long name: '{}'", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag::from_name()`] preserves case for already uppercase.
fn test_from_name_uppercase() -> bool {
    let tag: MmioTag = MmioTag::from_name("IOAPIC");
    let display_str: alloc::string::String = alloc::format!("{}", tag);

    if display_str != "IOAPIC  " {
        error!("unexpected display for uppercase name: '{}'", display_str);
        return false;
    }

    true
}

/// Tests if [`MmioTag`] equality works correctly.
fn test_equality() -> bool {
    let tag1: MmioTag = MmioTag::new(*b"TESTNAME");
    let tag2: MmioTag = MmioTag::new(*b"TESTNAME");
    let tag3: MmioTag = MmioTag::new(*b"DIFFNAME");

    if tag1 != tag2 {
        error!("identical tags should be equal");
        return false;
    }

    if tag1 == tag3 {
        error!("different tags should not be equal");
        return false;
    }

    true
}

/// Tests if [`MmioTag`] ordering works correctly.
fn test_ordering() -> bool {
    let tag_a: MmioTag = MmioTag::new(*b"AAAAAAAA");
    let tag_b: MmioTag = MmioTag::new(*b"BBBBBBBB");
    let tag_z: MmioTag = MmioTag::new(*b"ZZZZZZZZ");

    if !(tag_a < tag_b) {
        error!("tag_a should be less than tag_b");
        return false;
    }

    if !(tag_b < tag_z) {
        error!("tag_b should be less than tag_z");
        return false;
    }

    if !(tag_a < tag_z) {
        error!("tag_a should be less than tag_z");
        return false;
    }

    true
}

/// Runs all MmioTag tests.
pub fn test() -> bool {
    let mut passed: bool = true;

    info!("running MmioTag tests...");

    passed &= test_new_from_bytes();
    passed &= test_from_u64_hex_zero();
    passed &= test_from_u64_hex_nonzero();
    passed &= test_from_u64_ascii();
    passed &= test_from_u64_non_printable();
    passed &= test_from_name_short();
    passed &= test_from_name_long();
    passed &= test_from_name_uppercase();
    passed &= test_equality();
    passed &= test_ordering();

    if passed {
        info!("all MmioTag tests passed");
    } else {
        error!("some MmioTag tests failed");
    }

    passed
}
