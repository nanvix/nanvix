// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn accepts_normalized_paths() {
    for path in [
        "",
        "file",
        "directory/file",
        ".hidden",
        "directory/..hidden",
    ] {
        let resolved: FatResolvedPath = FatResolvedPath::new(String::from(path))
            .expect("normalized FAT path should be accepted");
        assert_eq!(resolved.as_str(), path);
        assert_eq!(resolved.into_string(), path);
    }
}

#[test]
fn rejects_unnormalized_paths() {
    for path in [
        "/file",
        ".",
        "..",
        "directory/./file",
        "directory/../file",
        "directory//file",
        "directory/",
        "file\0name",
    ] {
        assert_eq!(
            FatResolvedPath::new(String::from(path)),
            Err(Fat32Error::InvalidPath),
            "unnormalized FAT path should be rejected: {path:?}",
        );
    }
}
