// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_round_trip() {
    let mut builder: builder::MultibinBuilder = builder::MultibinBuilder::new();
    builder
        .add(vec![0x7f, b'E', b'L', b'F', 1, 2, 3, 4], "testd")
        .unwrap();
    builder
        .add(vec![0x7f, b'E', b'L', b'F', 5, 6, 7, 8], "procd;ENV=1")
        .unwrap();

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

    // No kernel args by default.
    assert_eq!(result.kernel_args_size(), 0);
}

#[test]
fn test_round_trip_with_kernel_args() {
    let mut builder: builder::MultibinBuilder = builder::MultibinBuilder::new();
    builder
        .add(vec![0x7f, b'E', b'L', b'F', 1, 2, 3, 4], "testd")
        .unwrap();
    builder
        .add(vec![0x7f, b'E', b'L', b'F', 5, 6, 7, 8], "procd;ENV=1")
        .unwrap();
    builder.set_kernel_args("snapshot verbose");

    let image: Vec<u8> = builder.build().expect("build should succeed");

    // Parse the image back.
    let result: ParseResult = parse(&image).expect("parse should succeed");
    assert_eq!(result.count(), 2);
    let ka_off: usize = result.kernel_args_offset();
    let ka_size: usize = result.kernel_args_size();
    assert_eq!(core::str::from_utf8(&image[ka_off..ka_off + ka_size]).unwrap(), "snapshot verbose");

    // Per-entry cmdlines should not contain kernel args.
    let e0: &ParsedEntry = result.get(0).expect("entry 0 should exist");
    assert_eq!(
        core::str::from_utf8(&image[e0.cmdline_offset..e0.cmdline_offset + e0.cmdline_size])
            .unwrap(),
        "testd"
    );
}

#[test]
fn test_iterator() {
    let mut builder: builder::MultibinBuilder = builder::MultibinBuilder::new();
    builder.add(vec![1, 2, 3], "a").unwrap();
    builder.add(vec![4, 5, 6], "b").unwrap();
    builder.add(vec![7, 8, 9], "c").unwrap();

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
