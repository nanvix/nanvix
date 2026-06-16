// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-image container format for consolidating multiple FAT32 filesystem images.
//!
//! This crate defines a binary format that packs a HEAD page followed by one or more
//! page-aligned sub-images. Both host (`std`) and guest (`no_std`) code can parse the
//! format; image *building* is gated behind the `std` feature.
//!
//! # Binary Layout
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │ HEAD page (PAGE_SIZE bytes)            │
//! │  ┌──────────────────────────────────┐  │
//! │  │ MultiImageHeader (36 bytes)      │  │
//! │  │ ImageEntry[0] (32 bytes)         │  │
//! │  │ ImageEntry[1] (32 bytes)         │  │
//! │  │ ...                              │  │
//! │  │ (zero-padded to PAGE_SIZE)       │  │
//! │  └──────────────────────────────────┘  │
//! ├────────────────────────────────────────┤
//! │ Image 0 data (page-aligned)           │
//! ├────────────────────────────────────────┤
//! │ Image 1 data (page-aligned)           │
//! └────────────────────────────────────────┘
//! ```

//==================================================================================================
// Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::all)]

//==================================================================================================
// Conditional Imports
//==================================================================================================

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::{
    fs,
    io::Write,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Re-Exports
//==================================================================================================

// Re-export well-known image tags from the config crate.
pub use ::config::region_tags::ROOTFS_MMIO_TAG;
use ::mmio_tag::{
    MmioTag,
    TAG_LENGTH,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Magic number identifying a multi-image container (`"MIMG"` in little-endian).
pub const HEADER_MAGIC: u32 = 0x4D49_4D47;

/// Current format version.
pub const HEADER_VERSION: u32 = 1;

/// Size of the HEAD page in bytes (must equal the target page size).
pub const HEAD_PAGE_SIZE: usize = 4096;

/// Maximum number of entries that fit in a single HEAD page.
///
/// `(HEAD_PAGE_SIZE - size_of::<MultiImageHeader>()) / size_of::<ImageEntry>()`
pub const MAX_ENTRIES: usize = (HEAD_PAGE_SIZE - HEADER_SIZE) / ENTRY_SIZE;

/// Size of [`MultiImageHeader`] in bytes.
pub const HEADER_SIZE: usize = core::mem::size_of::<MultiImageHeader>();

/// Size of [`ImageEntry`] in bytes.
pub const ENTRY_SIZE: usize = core::mem::size_of::<ImageEntry>();

/// `ImageEntry::flags` bit: sub-image is read-only.
pub const FLAG_READONLY: u32 = 1 << 0;

//==================================================================================================
// Structures
//==================================================================================================

/// Fixed header at the start of a multi-image container.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultiImageHeader {
    /// Must equal [`HEADER_MAGIC`].
    pub magic: u32,
    /// Must equal [`HEADER_VERSION`].
    pub version: u32,
    /// Number of [`ImageEntry`] records that follow.
    pub num_images: u32,
    /// Padding for alignment.
    pub _pad0: u32,
    /// Total size of the container in bytes (HEAD page + all sub-images with padding).
    pub total_size: u64,
    /// Reserved for future use (must be zero).
    pub reserved: [u8; 16],
}

/// Descriptor for a single sub-image inside the container.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ImageEntry {
    /// 8-byte tag identifying the image (e.g., [`ROOTFS_MMIO_TAG`]).
    pub tag: [u8; TAG_LENGTH],
    /// Byte offset from the start of the container to the sub-image data.
    pub offset: u64,
    /// Actual size of the sub-image data in bytes.
    pub size: u64,
    /// Flags. See [`FLAG_READONLY`].
    pub flags: u32,
    /// Padding for alignment.
    pub _pad: u32,
}

//==================================================================================================
// Error Types
//==================================================================================================

/// Errors returned by multi-image parsing functions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiImageError {
    /// The data slice is too short to contain the expected structure.
    BufferTooSmall,
    /// The magic field does not match [`HEADER_MAGIC`].
    BadMagic,
    /// The version field is not supported.
    UnsupportedVersion,
    /// `num_images` exceeds [`MAX_ENTRIES`].
    TooManyEntries,
    /// An entry offset or size would exceed the container bounds.
    EntryOutOfBounds,
    /// An entry offset is not page-aligned.
    EntryNotAligned,
    /// An I/O error occurred during building (std only).
    #[cfg(feature = "std")]
    Io,
}

#[cfg(feature = "std")]
impl std::fmt::Display for MultiImageError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::BadMagic => write!(f, "bad magic number"),
            Self::UnsupportedVersion => write!(f, "unsupported version"),
            Self::TooManyEntries => write!(f, "too many entries"),
            Self::EntryOutOfBounds => write!(f, "entry out of bounds"),
            Self::EntryNotAligned => write!(f, "entry not page-aligned"),
            Self::Io => write!(f, "I/O error"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MultiImageError {}

//==================================================================================================
// Parsing (no_std)
//==================================================================================================

/// Returns `true` if `data` starts with the multi-image magic number.
///
/// This is a quick probe that does **not** validate the rest of the header.
pub fn is_multiimage(data: &[u8]) -> bool {
    if data.len() < u32::BITS as usize / u8::BITS as usize {
        return false;
    }
    let magic: u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    magic == HEADER_MAGIC
}

/// Parses and validates the [`MultiImageHeader`] at the start of `data`.
///
/// # Errors
///
/// Returns an error if the buffer is too small, the magic is wrong, the version is
/// unsupported, or `num_images` exceeds [`MAX_ENTRIES`].
pub fn parse_header(data: &[u8]) -> Result<MultiImageHeader, MultiImageError> {
    if data.len() < HEADER_SIZE {
        return Err(MultiImageError::BufferTooSmall);
    }

    // SAFETY: We verified the buffer is large enough. We read field-by-field to
    // avoid alignment issues on platforms that require aligned reads.
    let header: MultiImageHeader = MultiImageHeader::from_bytes(data);

    if header.magic != HEADER_MAGIC {
        return Err(MultiImageError::BadMagic);
    }
    if header.version != HEADER_VERSION {
        return Err(MultiImageError::UnsupportedVersion);
    }
    if header.num_images as usize > MAX_ENTRIES {
        return Err(MultiImageError::TooManyEntries);
    }

    Ok(header)
}

/// Parses `count` [`ImageEntry`] records that follow the header in `data`.
///
/// `data` must start at the beginning of the container (i.e., the header).
///
/// # Errors
///
/// Returns an error if the buffer cannot hold `count` entries after the header, or if
/// any entry's offset/size would exceed the buffer.
pub fn parse_entries(data: &[u8], count: u32) -> Result<&[ImageEntry], MultiImageError> {
    let count_usize: usize = count as usize;
    let required: usize = HEADER_SIZE + count_usize * ENTRY_SIZE;
    if data.len() < required {
        return Err(MultiImageError::BufferTooSmall);
    }

    let entry_bytes: &[u8] = &data[HEADER_SIZE..required];
    let alignment: usize = core::mem::align_of::<ImageEntry>();
    if !(entry_bytes.as_ptr() as usize).is_multiple_of(alignment) {
        return Err(MultiImageError::BufferTooSmall);
    }

    // SAFETY: We checked that `entry_bytes` starts at an address aligned for
    // `ImageEntry`, and `entry_bytes` covers exactly `count_usize * ENTRY_SIZE`
    // bytes. Thus, reinterpreting it as a `[ImageEntry]` is sound.
    let (prefix, entries, suffix): (&[u8], &[ImageEntry], &[u8]) =
        unsafe { entry_bytes.align_to::<ImageEntry>() };
    if !prefix.is_empty() || !suffix.is_empty() || entries.len() != count_usize {
        return Err(MultiImageError::BufferTooSmall);
    }

    Ok(entries)
}

/// Finds the first entry whose `tag` matches the provided tag.
pub fn find_entry_by_tag<'a>(entries: &'a [ImageEntry], tag: &MmioTag) -> Option<&'a ImageEntry> {
    entries.iter().find(|e| &e.tag == tag.as_bytes())
}

/// Validates that all entries have page-aligned offsets and fit within `total_size`.
pub fn validate_entries(entries: &[ImageEntry], total_size: u64) -> Result<(), MultiImageError> {
    for entry in entries {
        if entry.offset % HEAD_PAGE_SIZE as u64 != 0 {
            return Err(MultiImageError::EntryNotAligned);
        }
        let end: u64 = entry
            .offset
            .checked_add(entry.size)
            .ok_or(MultiImageError::EntryOutOfBounds)?;
        if end > total_size {
            return Err(MultiImageError::EntryOutOfBounds);
        }
    }
    Ok(())
}

//==================================================================================================
// Internal Helpers
//==================================================================================================

impl MultiImageHeader {
    /// Reads a [`MultiImageHeader`] from a byte slice without requiring alignment.
    fn from_bytes(data: &[u8]) -> Self {
        let mut reserved: [u8; 16] = [0u8; 16];
        reserved.copy_from_slice(&data[24..40]);

        Self {
            magic: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            version: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            num_images: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            _pad0: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
            total_size: u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            reserved,
        }
    }
}

/// Rounds `value` up to the next multiple of [`HEAD_PAGE_SIZE`].
#[cfg(feature = "std")]
const fn page_align(value: usize) -> usize {
    let mask: usize = HEAD_PAGE_SIZE - 1;
    (value + mask) & !mask
}

//==================================================================================================
// Building (std only)
//==================================================================================================

#[cfg(feature = "std")]
/// Describes one sub-image to include when building a multi-image container.
pub struct ImageDescriptor<'a> {
    /// Tag identifying the image (e.g., [`ROOTFS_MMIO_TAG`]).
    pub tag: MmioTag,
    /// Path to the sub-image file on the host filesystem.
    pub path: &'a Path,
    /// Flags for this entry (e.g., [`FLAG_READONLY`]).
    pub flags: u32,
}

#[cfg(feature = "std")]
/// Describes the position and source of a single sub-image within a multi-image container.
///
/// Unlike [`ImageEntry`], which is part of the binary format, this struct carries the host
/// filesystem path so the VMM can open and map the original file directly (zero-copy).
pub struct ImageRegion {
    /// Tag identifying the image (e.g., [`ROOTFS_MMIO_TAG`]).
    pub tag: MmioTag,
    /// Path to the source file on the host filesystem.
    pub path: PathBuf,
    /// Byte offset from the start of the container where this sub-image is placed.
    pub offset: usize,
    /// Actual size of the sub-image data in bytes (not page-aligned).
    pub size: usize,
    /// Page-aligned size used for placement (includes trailing padding).
    pub page_aligned_size: usize,
    /// Flags for this entry (e.g., [`FLAG_READONLY`]).
    pub flags: u32,
}

#[cfg(feature = "std")]
/// A descriptor of a multi-image container layout that can be used by the VMM to map
/// sub-image files directly into guest memory without creating an intermediate concatenated file.
///
/// Produced by [`compute_multiimage_layout`] and consumed by the UserVM backends to write
/// the HEAD page and memory-map each sub-image file at its computed offset.
pub struct MultiImageLayout {
    /// The fully-built HEAD page (header + entries), ready to be written into guest memory.
    pub head_page: std::vec::Vec<u8>,
    /// Ordered list of sub-image regions with their source paths and computed offsets.
    pub regions: std::vec::Vec<ImageRegion>,
    /// Total size of the container in bytes (HEAD page + all page-aligned sub-images).
    pub total_size: usize,
}

#[cfg(feature = "std")]
/// Computes the multi-image container layout from the provided descriptors without reading
/// or writing any file data.
///
/// This function queries file metadata to obtain sizes, computes page-aligned offsets, and
/// builds the HEAD page bytes. The resulting [`MultiImageLayout`] allows the VMM to map each
/// source file directly into guest memory (zero-copy), avoiding the need for a concatenated
/// intermediate file on disk.
///
/// # Errors
///
/// Returns [`MultiImageError::TooManyEntries`] if more than [`MAX_ENTRIES`] images are
/// provided, or [`MultiImageError::Io`] on any metadata query failure.
pub fn compute_multiimage_layout(
    images: &[ImageDescriptor<'_>],
) -> Result<MultiImageLayout, MultiImageError> {
    if images.len() > MAX_ENTRIES {
        return Err(MultiImageError::TooManyEntries);
    }

    // Query file sizes from metadata (no file content reads).
    let mut file_sizes: std::vec::Vec<usize> = std::vec::Vec::with_capacity(images.len());
    for desc in images {
        let metadata: fs::Metadata = fs::metadata(desc.path).map_err(|_| MultiImageError::Io)?;
        file_sizes.push(metadata.len() as usize);
    }

    // Compute offsets. The first sub-image starts right after the HEAD page.
    let mut current_offset: usize = HEAD_PAGE_SIZE;
    let mut entries: std::vec::Vec<ImageEntry> = std::vec::Vec::with_capacity(images.len());
    let mut regions: std::vec::Vec<ImageRegion> = std::vec::Vec::with_capacity(images.len());

    for (i, desc) in images.iter().enumerate() {
        let size: usize = file_sizes[i];
        let aligned: usize = page_align(size);

        entries.push(ImageEntry {
            tag: *desc.tag.as_bytes(),
            offset: current_offset as u64,
            size: size as u64,
            flags: desc.flags,
            _pad: 0,
        });
        regions.push(ImageRegion {
            tag: desc.tag,
            path: desc.path.to_path_buf(),
            offset: current_offset,
            size,
            page_aligned_size: aligned,
            flags: desc.flags,
        });

        current_offset += aligned;
    }

    let total_size: usize = current_offset;

    // Build the HEAD page.
    let mut head_page: std::vec::Vec<u8> = std::vec![0u8; HEAD_PAGE_SIZE];
    write_header(
        &mut head_page,
        &MultiImageHeader {
            magic: HEADER_MAGIC,
            version: HEADER_VERSION,
            num_images: entries.len() as u32,
            _pad0: 0,
            total_size: total_size as u64,
            reserved: [0u8; 16],
        },
    );
    write_entries(&mut head_page, &entries);

    Ok(MultiImageLayout {
        head_page,
        regions,
        total_size,
    })
}

#[cfg(feature = "std")]
/// Builds a multi-image container from the provided descriptors and writes it to `output`.
///
/// Each sub-image file is read from disk, page-aligned, and concatenated after the HEAD page.
///
/// # Errors
///
/// Returns [`MultiImageError::TooManyEntries`] if more than [`MAX_ENTRIES`] images are
/// provided, or [`MultiImageError::Io`] on any I/O failure.
pub fn build_multiimage(
    images: &[ImageDescriptor<'_>],
    output: &Path,
) -> Result<(), MultiImageError> {
    if images.len() > MAX_ENTRIES {
        return Err(MultiImageError::TooManyEntries);
    }

    // Read all image files.
    let mut buffers: std::vec::Vec<std::vec::Vec<u8>> = std::vec::Vec::with_capacity(images.len());
    for desc in images {
        let data: std::vec::Vec<u8> = fs::read(desc.path).map_err(|_| MultiImageError::Io)?;
        buffers.push(data);
    }

    // Compute offsets. The first sub-image starts right after the HEAD page.
    let mut current_offset: usize = HEAD_PAGE_SIZE;
    let mut entries: std::vec::Vec<ImageEntry> = std::vec::Vec::with_capacity(images.len());

    for (i, desc) in images.iter().enumerate() {
        let size: usize = buffers[i].len();
        entries.push(ImageEntry {
            tag: *desc.tag.as_bytes(),
            offset: current_offset as u64,
            size: size as u64,
            flags: desc.flags,
            _pad: 0,
        });
        current_offset += page_align(size);
    }

    let total_size: u64 = current_offset as u64;

    // Build the HEAD page.
    let mut head: std::vec::Vec<u8> = std::vec![0u8; HEAD_PAGE_SIZE];
    write_header(
        &mut head,
        &MultiImageHeader {
            magic: HEADER_MAGIC,
            version: HEADER_VERSION,
            num_images: entries.len() as u32,
            _pad0: 0,
            total_size,
            reserved: [0u8; 16],
        },
    );
    write_entries(&mut head, &entries);

    // Write the output file.
    let mut file: fs::File = fs::File::create(output).map_err(|_| MultiImageError::Io)?;
    file.write_all(&head).map_err(|_| MultiImageError::Io)?;

    for (i, buf) in buffers.iter().enumerate() {
        file.write_all(buf).map_err(|_| MultiImageError::Io)?;
        // Pad to page boundary.
        let aligned: usize = page_align(buf.len());
        let padding: usize = aligned - buf.len();
        if padding > 0 {
            let zeros: std::vec::Vec<u8> = std::vec![0u8; padding];
            file.write_all(&zeros).map_err(|_| MultiImageError::Io)?;
        }
        // Suppress unused variable warning.
        let _ = &entries[i];
    }

    Ok(())
}

#[cfg(feature = "std")]
/// Serialises a [`MultiImageHeader`] into the first [`HEADER_SIZE`] bytes of `buf`.
fn write_header(buf: &mut [u8], header: &MultiImageHeader) {
    buf[0..4].copy_from_slice(&header.magic.to_le_bytes());
    buf[4..8].copy_from_slice(&header.version.to_le_bytes());
    buf[8..12].copy_from_slice(&header.num_images.to_le_bytes());
    buf[12..16].copy_from_slice(&header._pad0.to_le_bytes());
    buf[16..24].copy_from_slice(&header.total_size.to_le_bytes());
    buf[24..40].copy_from_slice(&header.reserved);
}

#[cfg(feature = "std")]
/// Serialises a slice of [`ImageEntry`] records into `buf` starting at [`HEADER_SIZE`].
fn write_entries(buf: &mut [u8], entries: &[ImageEntry]) {
    let mut offset: usize = HEADER_SIZE;
    for entry in entries {
        buf[offset..offset + 8].copy_from_slice(&entry.tag);
        buf[offset + 8..offset + 16].copy_from_slice(&entry.offset.to_le_bytes());
        buf[offset + 16..offset + 24].copy_from_slice(&entry.size.to_le_bytes());
        buf[offset + 24..offset + 28].copy_from_slice(&entry.flags.to_le_bytes());
        buf[offset + 28..offset + 32].copy_from_slice(&entry._pad.to_le_bytes());
        offset += ENTRY_SIZE;
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_is_multiimage_valid() {
        let mut buf: [u8; 4] = [0u8; 4];
        buf.copy_from_slice(&HEADER_MAGIC.to_le_bytes());
        assert!(is_multiimage(&buf));
    }

    #[test]
    fn test_is_multiimage_invalid() {
        assert!(!is_multiimage(&[0u8; 4]));
        assert!(!is_multiimage(&[0u8; 2]));
        assert!(!is_multiimage(&[]));
    }

    #[test]
    fn test_parse_header_valid() {
        let mut buf: [u8; HEAD_PAGE_SIZE] = [0u8; HEAD_PAGE_SIZE];
        write_header(
            &mut buf,
            &MultiImageHeader {
                magic: HEADER_MAGIC,
                version: HEADER_VERSION,
                num_images: 2,
                _pad0: 0,
                total_size: HEAD_PAGE_SIZE as u64 + 8192,
                reserved: [0u8; 16],
            },
        );
        let header: MultiImageHeader = parse_header(&buf).expect("parse_header failed");
        assert_eq!(header.magic, HEADER_MAGIC);
        assert_eq!(header.version, HEADER_VERSION);
        assert_eq!(header.num_images, 2);
        assert_eq!(header.total_size, HEAD_PAGE_SIZE as u64 + 8192);
    }

    #[test]
    fn test_parse_header_bad_magic() {
        let buf: [u8; HEADER_SIZE] = [0u8; HEADER_SIZE];
        assert_eq!(parse_header(&buf), Err(MultiImageError::BadMagic));
    }

    #[test]
    fn test_parse_header_buffer_too_small() {
        let buf: [u8; 4] = HEADER_MAGIC.to_le_bytes();
        assert_eq!(parse_header(&buf), Err(MultiImageError::BufferTooSmall));
    }

    #[test]
    fn test_roundtrip_build_and_parse() {
        // Create two temporary image files.
        let dir: tempfile::TempDir = tempfile::tempdir().expect("tempdir");
        let img1_path = dir.path().join("img1.bin");
        let img2_path = dir.path().join("img2.bin");
        let output_path = dir.path().join("unified.bin");

        let img1_data: std::vec::Vec<u8> = std::vec![0xAA; 5000]; // Not page-aligned
        let img2_data: std::vec::Vec<u8> = std::vec![0xBB; 4096]; // Exactly one page

        fs::write(&img1_path, &img1_data).expect("write img1");
        fs::write(&img2_path, &img2_data).expect("write img2");

        let descriptors: [ImageDescriptor<'_>; 2] = [
            ImageDescriptor {
                tag: ROOTFS_MMIO_TAG,
                path: &img1_path,
                flags: 0,
            },
            ImageDescriptor {
                tag: MmioTag::new(*b"TESTIMG "),
                path: &img2_path,
                flags: FLAG_READONLY,
            },
        ];

        build_multiimage(&descriptors, &output_path).expect("build_multiimage");

        // Read back and parse.
        let mut file: fs::File = fs::File::open(&output_path).expect("open unified");
        let mut data: std::vec::Vec<u8> = std::vec::Vec::new();
        file.read_to_end(&mut data).expect("read unified");

        assert!(is_multiimage(&data));
        let header: MultiImageHeader = parse_header(&data).expect("parse_header");
        assert_eq!(header.num_images, 2);

        let entries: &[ImageEntry] =
            parse_entries(&data, header.num_images).expect("parse_entries");
        assert_eq!(entries.len(), 2);

        // Validate ROOTFS entry.
        let rootfs: &ImageEntry =
            find_entry_by_tag(entries, &ROOTFS_MMIO_TAG).expect("ROOTFS not found");
        assert_eq!(rootfs.offset as usize, HEAD_PAGE_SIZE);
        assert_eq!(rootfs.size as usize, 5000);
        assert_eq!(rootfs.flags, 0);
        let rootfs_data: &[u8] =
            &data[rootfs.offset as usize..rootfs.offset as usize + rootfs.size as usize];
        assert!(rootfs_data.iter().all(|&b| b == 0xAA));

        // Validate TESTIMG entry.
        let testimg: &ImageEntry =
            find_entry_by_tag(entries, &MmioTag::new(*b"TESTIMG ")).expect("TESTIMG not found");
        assert_eq!(testimg.offset as usize, HEAD_PAGE_SIZE + page_align(5000));
        assert_eq!(testimg.size as usize, 4096);
        assert_eq!(testimg.flags, FLAG_READONLY);
        let testimg_data: &[u8] =
            &data[testimg.offset as usize..testimg.offset as usize + testimg.size as usize];
        assert!(testimg_data.iter().all(|&b| b == 0xBB));

        // Validate entries.
        validate_entries(entries, header.total_size).expect("validate_entries");
    }

    #[test]
    fn test_find_entry_by_tag_missing() {
        let entries: [ImageEntry; 1] = [ImageEntry {
            tag: *ROOTFS_MMIO_TAG.as_bytes(),
            offset: HEAD_PAGE_SIZE as u64,
            size: 4096,
            flags: 0,
            _pad: 0,
        }];
        assert!(find_entry_by_tag(&entries, &MmioTag::new(*b"MISSING ")).is_none());
    }

    #[test]
    fn test_validate_entries_out_of_bounds() {
        let entries: [ImageEntry; 1] = [ImageEntry {
            tag: *ROOTFS_MMIO_TAG.as_bytes(),
            offset: HEAD_PAGE_SIZE as u64,
            size: 99999,
            flags: 0,
            _pad: 0,
        }];
        assert_eq!(
            validate_entries(&entries, HEAD_PAGE_SIZE as u64 + 4096),
            Err(MultiImageError::EntryOutOfBounds)
        );
    }

    #[test]
    fn test_validate_entries_not_aligned() {
        let entries: [ImageEntry; 1] = [ImageEntry {
            tag: *ROOTFS_MMIO_TAG.as_bytes(),
            offset: 100, // Not page-aligned
            size: 4096,
            flags: 0,
            _pad: 0,
        }];
        assert_eq!(
            validate_entries(&entries, HEAD_PAGE_SIZE as u64 + 8192),
            Err(MultiImageError::EntryNotAligned)
        );
    }

    #[test]
    fn test_header_and_entry_sizes() {
        // Ensure the structures have the expected sizes for the binary format.
        assert_eq!(HEADER_SIZE, 40);
        assert_eq!(ENTRY_SIZE, 32);
        // At least 126 entries fit in a single HEAD page.
        assert!(MAX_ENTRIES >= 126);
    }
}
