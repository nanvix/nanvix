// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn construction_selects_explicit_anchor_state() {
    for (dirfd, path, expected) in [
        (17, "/absolute/../path", PathAnchor::Absolute(17)),
        (AT_FDCWD, "relative/path", PathAnchor::CurrentDirectory),
        (42, "relative/./path", PathAnchor::Directory(42)),
    ] {
        let anchored: AnchoredPath = AnchoredPath::new(dirfd, String::from(path))
            .expect("valid raw path should be accepted");
        assert_eq!(anchored.anchor, expected);
        assert_eq!(anchored.dirfd(), dirfd);
        assert_eq!(anchored.path(), path);
    }
}

#[test]
fn construction_rejects_invalid_raw_paths() {
    assert_eq!(AnchoredPath::new(AT_FDCWD, String::new()), Err(Fat32Error::NotFound),);
    for (dirfd, path) in [
        (-99, "/invalid\0/../absolute"),
        (AT_FDCWD, "invalid\0/../relative"),
    ] {
        assert_eq!(AnchoredPath::new(dirfd, String::from(path)), Err(Fat32Error::InvalidPath),);
    }
}

#[test]
fn absolute_path_ignores_directory_descriptor() {
    let path: AnchoredPath = AnchoredPath::new(-99, String::from("/absolute/../path"))
        .expect("valid raw path should be accepted");
    let result: String = path
        .into_absolute()
        .expect("absolute path should not inspect its directory descriptor");
    assert_eq!(result, "/absolute/../path");
}

#[test]
fn missing_directory_descriptor_is_rejected() {
    let path: AnchoredPath = AnchoredPath::new(c_int::MAX, String::from("relative/path"))
        .expect("valid raw path should be accepted");
    assert_eq!(path.into_absolute(), Err(Fat32Error::InvalidFd));
}
