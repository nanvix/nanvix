// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Constants
//==================================================================================================

/// Magic number identifying a multibinary image: b"NVMB".
pub const MAGIC: [u8; 4] = *b"NVMB";

/// Current format version.
pub const VERSION: u32 = 1;

/// Size of the image header in bytes.
pub const HEADER_SIZE: usize = core::mem::size_of::<MultibinHeader>();

/// Size of a single entry descriptor in bytes.
pub const ENTRY_SIZE: usize = core::mem::size_of::<MultibinEntry>();

/// Page size used for alignment of binary data.
pub const PAGE_SIZE: usize = 4096;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Header at the start of every multibinary image.
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MultibinHeader {
    /// Magic number (`NVMB`).
    pub magic: [u8; 4],
    /// Format version.
    pub version: u32,
    /// Number of entries in the image.
    pub num_entries: u32,
    /// Reserved for future use — must be zero.
    pub reserved: u32,
}

///
/// # Description
///
/// Descriptor for a single binary packed inside the image.
///
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MultibinEntry {
    /// Byte offset from the start of the image to the ELF binary data.
    pub offset: u32,
    /// Size of the ELF binary data in bytes.
    pub size: u32,
    /// Byte offset from the start of the image to the command-line string.
    pub cmdline_offset: u32,
    /// Length of the command-line string in bytes (UTF-8, not null-terminated).
    pub cmdline_size: u32,
}

///
/// # Description
///
/// A parsed entry referencing data within a multibinary image.
///
#[derive(Clone, Copy, Debug)]
pub struct ParsedEntry {
    /// Byte offset from the start of the image to the ELF binary data.
    pub offset: usize,
    /// Size of the ELF binary data in bytes.
    pub size: usize,
    /// Byte offset from the start of the image to the command-line string.
    pub cmdline_offset: usize,
    /// Length of the command-line string in bytes.
    pub cmdline_size: usize,
}

//==================================================================================================
// Parser (no_std)
//==================================================================================================

///
/// # Description
///
/// Maximum number of entries supported in a single multibinary image.
///
const MAX_ENTRIES: usize = 32;

///
/// # Description
///
/// Result of parsing a multibinary image header.
///
#[derive(Debug)]
pub struct ParseResult {
    /// Parsed entries.
    entries: [ParsedEntry; MAX_ENTRIES],
    /// Number of valid entries.
    count: usize,
}

impl ParseResult {
    ///
    /// # Description
    ///
    /// Returns the number of entries in the parsed image.
    ///
    pub fn count(&self) -> usize {
        self.count
    }

    ///
    /// # Description
    ///
    /// Returns the entry at the given index.
    ///
    /// # Parameters
    ///
    /// - `index`: Index of the entry to retrieve.
    ///
    /// # Returns
    ///
    /// The parsed entry at the given index, or `None` if the index is out of bounds.
    ///
    pub fn get(&self, index: usize) -> Option<&ParsedEntry> {
        if index < self.count {
            Some(&self.entries[index])
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Returns an iterator over all valid entries.
    ///
    pub fn iter(&self) -> ParseResultIter<'_> {
        ParseResultIter {
            result: self,
            index: 0,
        }
    }
}

///
/// # Description
///
/// Iterator over parsed multibinary entries.
///
pub struct ParseResultIter<'a> {
    result: &'a ParseResult,
    index: usize,
}

impl<'a> Iterator for ParseResultIter<'a> {
    type Item = &'a ParsedEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let entry: Option<&'a ParsedEntry> = self.result.get(self.index);
        if entry.is_some() {
            self.index += 1;
        }
        entry
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads a little-endian `u32` from a byte slice at the given offset.
///
fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

///
/// # Description
///
/// Parses a multibinary image from a byte slice.
///
/// Validates the header magic and version, then extracts entry descriptors. Does not copy any
/// binary or command-line data — the returned [`ParseResult`] contains offsets into the original
/// `data` slice.
///
/// # Parameters
///
/// - `data`: The raw bytes of the multibinary image.
///
/// # Returns
///
/// A [`ParseResult`] containing the parsed entries on success, or an [`error::Error`] if the
/// image is malformed.
///
pub fn parse(data: &[u8]) -> Result<ParseResult, error::Error> {
    // Validate minimum size for the header.
    if data.len() < HEADER_SIZE {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "multibinary image too small for header",
        ));
    }

    // Validate magic number.
    if data[0..4] != MAGIC {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "invalid multibinary magic number",
        ));
    }

    // Validate version.
    let version: u32 = read_u32(data, 4);
    if version != VERSION {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "unsupported multibinary version",
        ));
    }

    let num_entries: u32 = read_u32(data, 8);
    let num_entries_usize: usize = num_entries as usize;

    // Validate reserved field is zero.
    let reserved: u32 = read_u32(data, 12);
    if reserved != 0 {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "multibinary reserved field is non-zero",
        ));
    }

    // Validate entry count.
    if num_entries_usize == 0 {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "multibinary image has zero entries",
        ));
    }
    if num_entries_usize > MAX_ENTRIES {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "multibinary image has too many entries",
        ));
    }

    // Validate that the image is large enough for all entry descriptors.
    let entries_end: usize = HEADER_SIZE + num_entries_usize * ENTRY_SIZE;
    if data.len() < entries_end {
        return Err(error::Error::new(
            error::ErrorCode::InvalidArgument,
            "multibinary image too small for entries",
        ));
    }

    // Parse entry descriptors.
    let mut entries: [ParsedEntry; MAX_ENTRIES] = [ParsedEntry {
        offset: 0,
        size: 0,
        cmdline_offset: 0,
        cmdline_size: 0,
    }; MAX_ENTRIES];

    for (i, entry) in entries.iter_mut().enumerate().take(num_entries_usize) {
        let base: usize = HEADER_SIZE + i * ENTRY_SIZE;
        let offset: usize = read_u32(data, base) as usize;
        let size: usize = read_u32(data, base + 4) as usize;
        let cmdline_offset: usize = read_u32(data, base + 8) as usize;
        let cmdline_size: usize = read_u32(data, base + 12) as usize;

        // Validate that binary data is within bounds.
        if offset.checked_add(size).is_none_or(|end| end > data.len()) {
            return Err(error::Error::new(
                error::ErrorCode::InvalidArgument,
                "multibinary entry binary data out of bounds",
            ));
        }

        // Validate that command-line data is within bounds.
        if cmdline_offset
            .checked_add(cmdline_size)
            .is_none_or(|end| end > data.len())
        {
            return Err(error::Error::new(
                error::ErrorCode::InvalidArgument,
                "multibinary entry cmdline data out of bounds",
            ));
        }

        *entry = ParsedEntry {
            offset,
            size,
            cmdline_offset,
            cmdline_size,
        };
    }

    Ok(ParseResult {
        entries,
        count: num_entries_usize,
    })
}

//==================================================================================================
// Builder (std only)
//==================================================================================================

#[cfg(feature = "std")]
pub mod builder {
    use super::*;

    extern crate std;
    use std::vec::Vec;

    /// A single binary entry to be packed into the image.
    struct BuildEntry {
        elf_data: Vec<u8>,
        cmdline: Vec<u8>,
    }

    ///
    /// # Description
    ///
    /// Builds a multibinary image from individual ELF binaries and their command lines.
    ///
    pub struct MultibinBuilder {
        entries: Vec<BuildEntry>,
    }

    impl MultibinBuilder {
        ///
        /// # Description
        ///
        /// Creates a new empty builder.
        ///
        pub fn new() -> Self {
            Self {
                entries: Vec::new(),
            }
        }

        ///
        /// # Description
        ///
        /// Adds a binary with its command line to the image.
        ///
        /// # Parameters
        ///
        /// - `elf_data`: Raw bytes of the ELF binary.
        /// - `cmdline`: Command-line string for this binary (UTF-8).
        ///
        pub fn add(&mut self, elf_data: Vec<u8>, cmdline: &str) -> &mut Self {
            self.entries.push(BuildEntry {
                elf_data,
                cmdline: cmdline.as_bytes().to_vec(),
            });
            self
        }

        ///
        /// # Description
        ///
        /// Builds the multibinary image.
        ///
        /// # Returns
        ///
        /// The serialized multibinary image as a byte vector.
        ///
        pub fn build(&self) -> Result<Vec<u8>, error::Error> {
            let num_entries: usize = self.entries.len();
            if num_entries > MAX_ENTRIES {
                return Err(error::Error::new(
                    error::ErrorCode::InvalidArgument,
                    "too many entries for multibinary image",
                ));
            }

            // Compute the size of the header + entry descriptors.
            let descriptors_size: usize = HEADER_SIZE + num_entries * ENTRY_SIZE;

            // Lay out command-line strings immediately after descriptors.
            let mut cmdline_offsets: Vec<usize> = Vec::with_capacity(num_entries);
            let mut cursor: usize = descriptors_size;
            for entry in &self.entries {
                cmdline_offsets.push(cursor);
                cursor += entry.cmdline.len();
            }

            // Align to page boundary for first binary.
            let first_binary_offset: usize = align_up(cursor, PAGE_SIZE)?;

            // Lay out binary data with page alignment.
            let mut binary_offsets: Vec<usize> = Vec::with_capacity(num_entries);
            let mut binary_cursor: usize = first_binary_offset;
            for entry in &self.entries {
                binary_offsets.push(binary_cursor);
                binary_cursor += entry.elf_data.len();
                binary_cursor = align_up(binary_cursor, PAGE_SIZE)?;
            }

            let total_size: usize = binary_cursor;

            // Build the image.
            let mut image: Vec<u8> = std::vec![0u8; total_size];

            // Write header.
            image[0..4].copy_from_slice(&MAGIC);
            image[4..8].copy_from_slice(&(VERSION).to_le_bytes());
            image[8..12].copy_from_slice(&(num_entries as u32).to_le_bytes());
            image[12..16].copy_from_slice(&0u32.to_le_bytes());

            // Write entry descriptors.
            for i in 0..num_entries {
                let base: usize = HEADER_SIZE + i * ENTRY_SIZE;
                if binary_offsets[i] > u32::MAX as usize
                    || self.entries[i].elf_data.len() > u32::MAX as usize
                    || cmdline_offsets[i] > u32::MAX as usize
                    || self.entries[i].cmdline.len() > u32::MAX as usize
                {
                    return Err(error::Error::new(
                        error::ErrorCode::InvalidArgument,
                        "multibinary entry offset or size exceeds u32",
                    ));
                }
                image[base..base + 4].copy_from_slice(&(binary_offsets[i] as u32).to_le_bytes());
                image[base + 4..base + 8]
                    .copy_from_slice(&(self.entries[i].elf_data.len() as u32).to_le_bytes());
                image[base + 8..base + 12]
                    .copy_from_slice(&(cmdline_offsets[i] as u32).to_le_bytes());
                image[base + 12..base + 16]
                    .copy_from_slice(&(self.entries[i].cmdline.len() as u32).to_le_bytes());
            }

            // Write command-line strings.
            for (i, entry) in self.entries.iter().enumerate() {
                let off: usize = cmdline_offsets[i];
                image[off..off + entry.cmdline.len()].copy_from_slice(&entry.cmdline);
            }

            // Write binary data.
            for (i, entry) in self.entries.iter().enumerate() {
                let off: usize = binary_offsets[i];
                image[off..off + entry.elf_data.len()].copy_from_slice(&entry.elf_data);
            }

            Ok(image)
        }
    }

    impl Default for MultibinBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Aligns `value` up to the next multiple of `align`.
    fn align_up(value: usize, align: usize) -> Result<usize, error::Error> {
        let mask: usize = align - 1;
        match value.checked_add(mask) {
            Some(v) => Ok(v & !mask),
            None => Err(error::Error::new(error::ErrorCode::InvalidArgument, "align_up overflow")),
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        let mut builder: builder::MultibinBuilder = builder::MultibinBuilder::new();
        builder.add(vec![0x7f, b'E', b'L', b'F', 1, 2, 3, 4], "testd");
        builder.add(vec![0x7f, b'E', b'L', b'F', 5, 6, 7, 8], "procd;ENV=1");

        let image: Vec<u8> = builder.build().expect("build should succeed");

        // Parse the image back.
        let result: ParseResult = parse(&image).expect("parse should succeed");
        assert_eq!(result.count(), 2);

        // Validate first entry.
        let e0: &ParsedEntry = result.get(0).expect("entry 0 should exist");
        assert_eq!(&image[e0.offset..e0.offset + e0.size], &[0x7f, b'E', b'L', b'F', 1, 2, 3, 4]);
        assert_eq!(
            core::str::from_utf8(&image[e0.cmdline_offset..e0.cmdline_offset + e0.cmdline_size])
                .unwrap(),
            "testd"
        );

        // Validate second entry.
        let e1: &ParsedEntry = result.get(1).expect("entry 1 should exist");
        assert_eq!(&image[e1.offset..e1.offset + e1.size], &[0x7f, b'E', b'L', b'F', 5, 6, 7, 8]);
        assert_eq!(
            core::str::from_utf8(&image[e1.cmdline_offset..e1.cmdline_offset + e1.cmdline_size])
                .unwrap(),
            "procd;ENV=1"
        );

        // Validate page alignment of binary data.
        assert_eq!(e0.offset % PAGE_SIZE, 0);
        assert_eq!(e1.offset % PAGE_SIZE, 0);
    }

    #[test]
    fn test_iterator() {
        let mut builder: builder::MultibinBuilder = builder::MultibinBuilder::new();
        builder.add(vec![1, 2, 3], "a");
        builder.add(vec![4, 5, 6], "b");
        builder.add(vec![7, 8, 9], "c");

        let image: Vec<u8> = builder.build().expect("build should succeed");
        let result: ParseResult = parse(&image).expect("parse should succeed");

        let cmdlines: Vec<&str> = result
            .iter()
            .map(|e: &ParsedEntry| {
                core::str::from_utf8(&image[e.cmdline_offset..e.cmdline_offset + e.cmdline_size])
                    .unwrap()
            })
            .collect();
        assert_eq!(cmdlines, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_invalid_magic() {
        let data: Vec<u8> = vec![0u8; HEADER_SIZE];
        assert!(parse(&data).is_err());
    }

    #[test]
    fn test_parse_too_small() {
        let data: Vec<u8> = vec![0u8; 4];
        assert!(parse(&data).is_err());
    }

    #[test]
    fn test_parse_zero_entries() {
        let mut data: Vec<u8> = vec![0u8; HEADER_SIZE];
        data[0..4].copy_from_slice(&MAGIC);
        data[4..8].copy_from_slice(&VERSION.to_le_bytes());
        data[8..12].copy_from_slice(&0u32.to_le_bytes());
        assert!(parse(&data).is_err());
    }
}
