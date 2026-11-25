// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn multibinary_cmdline_is_optional_but_explicit_empty_replaces_baked_args() {
    assert_eq!(Guest::initrd_cmdline("app.initrd", true, None).expect("no override"), None);
    assert_eq!(
        Guest::initrd_cmdline("app.initrd", true, Some("")).expect("empty override"),
        Some("app.initrd ".to_string())
    );
}

#[test]
fn initrd_cmdline_preserves_argument_and_environment_encoding() {
    for is_multibinary in [false, true] {
        assert_eq!(
            Guest::initrd_cmdline("app.initrd", is_multibinary, Some(r"hello a\;b;NEW=value"))
                .expect("encoded arguments"),
            Some(r"app.initrd hello a\;b;NEW=value".to_string())
        );
        assert_eq!(
            Guest::initrd_cmdline("app.initrd", is_multibinary, Some(";NEW=value"))
                .expect("environment-only override"),
            Some("app.initrd ;NEW=value".to_string())
        );
    }
}

#[test]
fn single_binary_cmdline_keeps_filename_without_arguments() {
    let path: ::std::path::PathBuf = ::std::path::Path::new("bin").join("app.elf");
    assert_eq!(
        Guest::initrd_cmdline(path.to_str().expect("UTF-8 test path"), false, None)
            .expect("single binary"),
        Some("app.elf".to_string())
    );
}

#[test]
fn initrd_cmdline_escapes_semicolons_in_basename() {
    for is_multibinary in [false, true] {
        assert_eq!(
            Guest::initrd_cmdline("app;release.initrd", is_multibinary, Some("x;NEW=value"))
                .expect("escaped basename"),
            Some(r"app\;release.initrd x;NEW=value".to_string())
        );
    }
}

#[test]
fn initrd_cmdline_rejects_spaces_only_when_basename_is_packed() {
    for is_multibinary in [false, true] {
        let error: anyhow::Error =
            Guest::initrd_cmdline("my app.initrd", is_multibinary, Some("x"))
                .expect_err("a space would split argv[0]");
        assert!(
            error
                .to_string()
                .contains("initrd basename cannot contain spaces")
        );
    }
    assert_eq!(
        Guest::initrd_cmdline("my app.initrd", true, None).expect("baked command line"),
        None
    );
    let path: ::std::path::PathBuf = ::std::path::Path::new("my directory").join("app.initrd");
    assert_eq!(
        Guest::initrd_cmdline(path.to_str().expect("UTF-8 test path"), true, Some("x"))
            .expect("spaces outside the basename are allowed"),
        Some("app.initrd x".to_string())
    );
}
