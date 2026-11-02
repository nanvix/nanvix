// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::path::PathBuf;

#[test]
fn executable_directory_returns_parent_directory() {
    let bin_directory: PathBuf = PathBuf::from("sysroot").join("bin");
    let executable_path: PathBuf = bin_directory.join("nanvixd.elf");
    assert_eq!(executable_directory(&executable_path).as_deref(), bin_directory.to_str());
}

#[test]
fn executable_directory_rejects_bare_file_name() {
    assert_eq!(executable_directory(Path::new("nanvixd.elf")), None);
}

#[test]
fn executable_directory_rejects_root_directory() {
    assert_eq!(executable_directory(Path::new("/")), None);
}

#[cfg(unix)]
#[test]
fn executable_directory_rejects_non_utf8_directory() {
    use ::std::{
        ffi::OsStr,
        os::unix::ffi::OsStrExt,
    };

    let executable_path: &Path = Path::new(OsStr::from_bytes(b"/opt/\xff/nanvixd.elf"));
    assert_eq!(executable_directory(executable_path), None);
}

#[test]
fn default_binary_directory_is_executable_directory() {
    let executable_path: PathBuf =
        ::std::env::current_exe().expect("failed to get current executable path");
    let expected: &Path = executable_path
        .parent()
        .expect("current executable has no parent directory");
    assert_eq!(Path::new(&default_binary_directory()), expected);
}
