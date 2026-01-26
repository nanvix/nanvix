// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::fmt::{
    self,
    Debug,
    Display,
    Formatter,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Length in bytes of an MMIO tag. This value was chosen so it can be stored in a u64.
pub const TAG_LENGTH: usize = 8;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Tag that uniquely identifies a memory-mapped I/O (MMIO) region. The tag is exactly
/// `TAG_LENGTH` characters long.
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MmioTag {
    /// Raw tag value with fixed length `TAG_LENGTH`.
    value: [u8; TAG_LENGTH],
}

//==================================================================================================
// Implementations
//==================================================================================================

impl MmioTag {
    ///
    /// # Description
    ///
    /// Creates a new MMIO tag from an 8-byte array.
    ///
    /// # Parameters
    ///
    /// - `value`: The 8-byte array that composes the tag.
    ///
    /// # Returns
    ///
    /// This function returns a new [`MmioTag`].
    ///
    pub const fn new(value: [u8; TAG_LENGTH]) -> Self {
        Self { value }
    }

    ///
    /// # Description
    ///
    /// Builds a tag from a 64-bit value encoded as eight hexadecimal characters.
    ///
    /// # Parameters
    ///
    /// - `value`: The 64-bit value to encode. Only the lower 32 bits (8 hex nibbles) are used
    ///   since `TAG_LENGTH` is 8 bytes.
    ///
    /// # Returns
    ///
    /// This function returns a new [`MmioTag`].
    ///
    pub fn from_u64_hex(value: u64) -> Self {
        let mut buf: [u8; TAG_LENGTH] = [b'0'; TAG_LENGTH];
        let mut val: u64 = value;
        let mut idx: usize = TAG_LENGTH;
        while idx > 0 {
            idx -= 1;
            let nibble: u8 = (val & 0xF) as u8;
            buf[idx] = Self::nibble_to_ascii(nibble);
            val >>= 4;
        }

        Self::new(buf)
    }

    ///
    /// # Description
    ///
    /// Builds a tag from a 64-bit value encoded either as printable ASCII bytes or hexadecimal
    /// digits. Printable ASCII input is preferred to support descriptive tags (e.g., `"RAMFS   "`).
    /// Non-printable sequences fall back to hexadecimal decoding so legacy numeric tags keep
    /// working.
    ///
    /// # Parameters
    ///
    /// - `value`: The 64-bit payload supplied by user space.
    ///
    /// # Returns
    ///
    /// This function returns a new [`MmioTag`].
    ///
    pub fn from_u64(value: u64) -> Self {
        let ascii_bytes: [u8; TAG_LENGTH] = value.to_be_bytes();
        let is_printable: bool = ascii_bytes.iter().all(|byte| matches!(byte, b' '..=b'~'));

        if is_printable {
            Self::new(ascii_bytes)
        } else {
            Self::from_u64_hex(value)
        }
    }

    ///
    /// # Description
    ///
    /// Derives a tag from a human-readable region name. The name is uppercased, truncated to
    /// `TAG_LENGTH`, and non-printable characters are replaced with underscores so it always
    /// yields a deterministic tag value.
    ///
    /// # Parameters
    ///
    /// - `name`: Region name string.
    ///
    /// # Returns
    ///
    /// A new [`MmioTag`] generated from the provided name.
    ///
    pub fn from_name(name: &str) -> Self {
        let mut bytes: [u8; TAG_LENGTH] = [b' '; TAG_LENGTH];
        let name_bytes: &[u8] = name.as_bytes();

        for idx in 0..core::cmp::min(TAG_LENGTH, name_bytes.len()) {
            bytes[idx] = Self::sanitize_ascii(name_bytes[idx]);
        }

        Self::new(bytes)
    }

    fn byte_to_char(byte: u8) -> char {
        // All u8 values (0-255) are valid Unicode scalar values, so this cast is safe.
        byte as char
    }

    fn nibble_to_ascii(n: u8) -> u8 {
        match n {
            0..=9 => b'0' + n,
            10..=15 => b'A' + (n - 10),
            _ => b'?',
        }
    }

    fn sanitize_ascii(byte: u8) -> u8 {
        match byte {
            b'a'..=b'z' => byte.to_ascii_uppercase(),
            b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' => byte,
            0x20..=0x7e => byte, // Printable ASCII range (includes space).
            _ => b'_',
        }
    }
}

impl Debug for MmioTag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        f.write_str("\"")?;
        for byte in self.value.iter() {
            let ch: char = Self::byte_to_char(*byte);
            ::core::fmt::write(f, format_args!("{ch}"))?;
        }
        f.write_str("\"")
    }
}

impl Display for MmioTag {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        for byte in self.value.iter() {
            let ch: char = Self::byte_to_char(*byte);
            ::core::fmt::write(f, format_args!("{ch}"))?;
        }
        Ok(())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(test)]
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= test::test();

    passed
}
