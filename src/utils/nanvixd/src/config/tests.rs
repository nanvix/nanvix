// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::path::PathBuf;
use ::tempfile::TempDir;

///
/// Writes a configuration file named `nanvixd.toml` to a directory and returns its path.
///
fn write_config_file(directory: &Path, contents: &str) -> PathBuf {
    let config_path: PathBuf = directory.join("nanvixd.toml");
    ::std::fs::write(&config_path, contents).expect("failed to write configuration file");
    config_path
}

#[test]
fn config_file_resolves_paths_against_its_directory() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_directory: PathBuf = tmp_dir.path().join("etc");
    ::std::fs::create_dir_all(config_directory.join("share")).expect("failed to create dirs");
    ::std::fs::write(config_directory.join("vm.snapshot"), b"snapshot")
        .expect("failed to write snapshot");
    let log_directory: PathBuf = tmp_dir.path().join("logs");
    let contents: String = [
        "bin_dir = \"bin\"".to_string(),
        "console_file = \"console.log\"".to_string(),
        "ramfs = \"rootfs.img\"".to_string(),
        "snapshot = \"vm.snapshot\"".to_string(),
        "mount = \"share\"".to_string(),
        format!("log_dir = '{}'", log_directory.display()),
    ]
    .join("\n");
    let config_path: PathBuf = write_config_file(&config_directory, &contents);

    let config_file: ConfigFile =
        ConfigFile::load(&config_path).expect("failed to load configuration file");
    let resolved = |name: &str| Some(config_directory.join(name).to_string_lossy().into_owned());
    assert_eq!(config_file.bin_dir, resolved("bin"));
    assert_eq!(config_file.console_file, resolved("console.log"));
    assert_eq!(config_file.ramfs, resolved("rootfs.img"));
    assert_eq!(config_file.snapshot, resolved("vm.snapshot"));
    assert_eq!(config_file.mount, resolved("share"));
    assert_eq!(config_file.log_dir.as_deref().map(Path::new), Some(log_directory.as_path()));
}

#[cfg(unix)]
#[test]
fn config_file_resolves_gateway_socket_path_against_its_directory() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: PathBuf =
        write_config_file(tmp_dir.path(), "gateway_sockaddr = \"gateway.sock\"\n");

    let config_file: ConfigFile =
        ConfigFile::load(&config_path).expect("failed to load configuration file");
    assert_eq!(
        config_file.gateway_sockaddr.as_deref().map(Path::new),
        Some(tmp_dir.path().join("gateway.sock").as_path())
    );
}

#[cfg(windows)]
#[test]
fn config_file_keeps_gateway_named_pipe_path() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: PathBuf =
        write_config_file(tmp_dir.path(), r"gateway_sockaddr = '\\.\pipe\nanvix-gw'");

    let config_file: ConfigFile =
        ConfigFile::load(&config_path).expect("failed to load configuration file");
    assert_eq!(config_file.gateway_sockaddr.as_deref(), Some(r"\\.\pipe\nanvix-gw"));
}

#[cfg(windows)]
#[test]
fn config_file_rejects_gateway_path_that_is_not_a_named_pipe() {
    for gateway_sockaddr in ["gateway.sock", r"C:\nanvix\gateway.sock", r"\\.\pipe\"] {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
        let contents: String = format!("gateway_sockaddr = '{gateway_sockaddr}'");
        let config_path: PathBuf = write_config_file(tmp_dir.path(), &contents);
        let error: ::anyhow::Error =
            ConfigFile::load(&config_path).expect_err("gateway path that is not a named pipe");
        let message: String = error.to_string();
        assert!(message.contains("named pipe path"), "unexpected error: {message}");
    }
}

#[test]
fn config_file_rejects_invalid_options() {
    let cases: [(&str, &str); 5] = [
        ("bin_directory = \"bin\"\n", "unknown field `bin_directory`"),
        ("log_to_stdout = true\nlog_dir = \"logs\"\n", "log_to_stdout and log_dir"),
        ("block_hosts = [\"8.8.8.8/40\"]\n", "invalid block_hosts entry"),
        ("snapshot = \"missing.snapshot\"\n", "snapshot path does not exist"),
        ("mount = \"nanvixd.toml\"\n", "mount path is not a directory"),
    ];
    for (contents, expected) in cases {
        let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
        let config_path: PathBuf = write_config_file(tmp_dir.path(), contents);
        let error: ::anyhow::Error =
            ConfigFile::load(&config_path).expect_err("invalid configuration file");
        let message: String = error.to_string();
        assert!(message.contains(expected), "unexpected error for {contents:?}: {message}");
        assert!(
            message.contains(&config_path.display().to_string()),
            "error does not name the configuration file: {message}"
        );
    }
}

#[test]
fn config_file_reports_missing_file() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: PathBuf = tmp_dir.path().join("missing.toml");
    let error: ::anyhow::Error =
        ConfigFile::load(&config_path).expect_err("missing configuration file");
    let message: String = error.to_string();
    assert!(message.contains("failed to read configuration file"), "unexpected error: {message}");
    assert!(message.contains(&config_path.display().to_string()), "unexpected error: {message}");
}

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
