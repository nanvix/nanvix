// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn multibinary_cmdline_is_optional_but_explicit_empty_replaces_baked_args() {
    assert_eq!(Guest::initrd_cmdline("app.initrd", true, None), None);
    assert_eq!(
        Guest::initrd_cmdline("app.initrd", true, Some("")),
        Some("app.initrd ".to_string())
    );
}

#[test]
fn initrd_cmdline_preserves_argument_and_environment_encoding() {
    for is_multibinary in [false, true] {
        assert_eq!(
            Guest::initrd_cmdline("app.initrd", is_multibinary, Some(r"hello a\;b;NEW=value")),
            Some(r"app.initrd hello a\;b;NEW=value".to_string())
        );
        assert_eq!(
            Guest::initrd_cmdline("app.initrd", is_multibinary, Some(";NEW=value")),
            Some("app.initrd ;NEW=value".to_string())
        );
    }
}

#[test]
fn single_binary_cmdline_keeps_filename_without_arguments() {
    let path: ::std::path::PathBuf = ::std::path::Path::new("bin").join("app.elf");
    assert_eq!(
        Guest::initrd_cmdline(path.to_str().expect("UTF-8 test path"), false, None),
        Some("app.elf".to_string())
    );
}
