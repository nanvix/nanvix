// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    normalize,
    normalize_lexically,
};
use ::alloc::string::String;

/// Checks that `normalizer` maps each path to its expected normal form.
fn check(normalizer: fn(&str) -> String, cases: &[(&str, &str)]) {
    for &(path, expected) in cases {
        assert_eq!(normalizer(path), expected, "normalizing {path:?}");
    }
}

#[test]
fn both_collapse_separators_and_dots() {
    let normalizers: [fn(&str) -> String; 2] = [normalize, normalize_lexically];
    for normalizer in normalizers {
        check(
            normalizer,
            &[
                ("/lib//./libfoo.so", "/lib/libfoo.so"),
                ("//lib/libfoo.so", "/lib/libfoo.so"),
                ("./lib/libfoo.so", "lib/libfoo.so"),
                (".//lib/./libfoo.so", "lib/libfoo.so"),
                ("/../lib", "/lib"),
                ("../lib", "../lib"),
                ("/", "/"),
                ("//", "/"),
                (".", "."),
                ("./.", "."),
                ("", ""),
            ],
        );
    }
}

#[test]
fn normalize_keeps_filesystem_dependent_syntax() {
    check(
        normalize,
        &[
            // The parent of a symbolic link's target need not hold the link.
            ("lib/../lib/libfoo.so", "lib/../lib/libfoo.so"),
            ("/mnt/link/..", "/mnt/link/.."),
            // A trailing separator, or a final `.`, requires a directory.
            ("lib/libfoo.so/", "lib/libfoo.so/"),
            ("lib/libfoo.so//", "lib/libfoo.so/"),
            ("lib/libfoo.so/.", "lib/libfoo.so/"),
            ("/lib/.", "/lib/"),
            ("./", "."),
        ],
    );
}

#[test]
fn normalize_lexically_folds_parents_and_trailing_separators() {
    check(
        normalize_lexically,
        &[
            ("/data/subdir/../file.txt", "/data/file.txt"),
            ("/data/../..", "/"),
            ("/data/subdir/", "/data/subdir"),
            ("/data/subdir/.", "/data/subdir"),
            ("a/..", "."),
            // A relative path keeps the `..` components that it cannot fold.
            ("a/b/../../../c", "../c"),
            ("../a/../..", "../.."),
        ],
    );
}
