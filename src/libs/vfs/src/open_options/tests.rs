// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that default OpenOptions has all flags false.
#[test]
fn open_options_default() {
    let opts: OpenOptions = OpenOptions::new();
    assert!(!opts.read, "read should default to false");
    assert!(!opts.write, "write should default to false");
    assert!(!opts.create, "create should default to false");
    assert!(!opts.create_new, "create_new should default to false");
    assert!(!opts.truncate, "truncate should default to false");
}

/// Tests builder method chaining.
#[test]
fn open_options_builder_chaining() {
    let opts: OpenOptions = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true);
    assert!(opts.read, "read should be true");
    assert!(opts.write, "write should be true");
    assert!(opts.create, "create should be true");
    assert!(opts.truncate, "truncate should be true");
}

/// Tests that truncate without write is rejected.
#[test]
fn open_options_truncate_requires_write() {
    let result = OpenOptions::new().truncate(true).open("/nonexistent");
    assert_eq!(
        result.expect_err("truncate without write should fail"),
        Fat32Error::InvalidArgument,
        "truncate without write should be rejected"
    );
}

/// Tests that create_new + create is rejected.
#[test]
fn open_options_create_new_excludes_create() {
    let result = OpenOptions::new()
        .write(true)
        .create(true)
        .create_new(true)
        .open("/nonexistent");
    assert_eq!(
        result.expect_err("create_new with create should fail"),
        Fat32Error::InvalidArgument,
        "create_new + create should be rejected"
    );
}

/// Tests that create_new + truncate is rejected.
#[test]
fn open_options_create_new_excludes_truncate() {
    let result = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create_new(true)
        .open("/nonexistent");
    assert_eq!(
        result.expect_err("create_new with truncate should fail"),
        Fat32Error::InvalidArgument,
        "create_new + truncate should be rejected"
    );
}

/// Tests that Default trait matches OpenOptions::new().
#[test]
fn open_options_implements_default() {
    let default: OpenOptions = OpenOptions::default();
    let new: OpenOptions = OpenOptions::new();
    assert_eq!(default.read, new.read);
    assert_eq!(default.write, new.write);
    assert_eq!(default.create, new.create);
    assert_eq!(default.create_new, new.create_new);
    assert_eq!(default.truncate, new.truncate);
}

/// Tests that OpenOptions implements Debug.
#[test]
fn open_options_debug() {
    let opts: OpenOptions = OpenOptions::new().read(true);
    let debug: alloc::string::String = alloc::format!("{opts:?}");
    assert!(debug.contains("OpenOptions"), "debug output should contain type name");
}
