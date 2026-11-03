// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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

    let entries: &[ImageEntry] = parse_entries(&data, header.num_images).expect("parse_entries");
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
