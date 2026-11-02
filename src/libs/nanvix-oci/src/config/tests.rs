// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

fn make_labels(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn parse_full_annotations() {
    let labels = make_labels(&[
        (annotations::OS, "nanvix"),
        (annotations::ARCH, "x86"),
        (annotations::INITRD_PATH, "/initrd/app.elf"),
        (annotations::INITRD_ARGS, "--verbose /config.txt"),
        (annotations::RAMFS_ROOT, "/ramfs"),
        (annotations::VERSION, "0.12.166"),
    ]);

    let config = NanvixImageConfig::from_annotations(&labels).unwrap();
    assert_eq!(config.initrd_path, "/initrd/app.elf");
    assert_eq!(config.initrd_args, vec!["--verbose", "/config.txt"]);
    assert_eq!(config.ramfs_root, Some("/ramfs".to_string()));
    assert_eq!(config.arch, "x86");
    assert_eq!(config.version, Some("0.12.166".to_string()));
}

#[test]
fn parse_minimal_annotations() {
    let labels = make_labels(&[
        (annotations::OS, "nanvix"),
        (annotations::INITRD_PATH, "/initrd/hello.elf"),
    ]);

    let config = NanvixImageConfig::from_annotations(&labels).unwrap();
    assert_eq!(config.initrd_path, "/initrd/hello.elf");
    assert!(config.initrd_args.is_empty());
    assert!(config.ramfs_root.is_none());
    assert_eq!(config.arch, "x86");
    assert!(config.version.is_none());
}

#[test]
fn not_nanvix_image() {
    let labels = make_labels(&[("other.label", "value")]);
    assert!(NanvixImageConfig::from_annotations(&labels).is_none());
}

#[test]
fn wrong_os() {
    let labels = make_labels(&[
        (annotations::OS, "linux"),
        (annotations::INITRD_PATH, "/initrd/app.elf"),
    ]);
    assert!(NanvixImageConfig::from_annotations(&labels).is_none());
}

#[test]
fn missing_initrd_path() {
    let labels = make_labels(&[(annotations::OS, "nanvix")]);
    assert!(NanvixImageConfig::from_annotations(&labels).is_none());
}

#[test]
fn empty_ramfs_root_treated_as_none() {
    let labels = make_labels(&[
        (annotations::OS, "nanvix"),
        (annotations::INITRD_PATH, "/initrd/app.elf"),
        (annotations::RAMFS_ROOT, ""),
    ]);

    let config = NanvixImageConfig::from_annotations(&labels).unwrap();
    assert!(config.ramfs_root.is_none());
}
