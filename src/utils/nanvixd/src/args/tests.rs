// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::nanvix::sandbox_config::HostAction;
use ::std::path::{
    Path,
    PathBuf,
};
use ::tempfile::TempDir;

fn argv(extras: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = vec!["nanvixd".to_string()];
    for s in extras {
        v.push((*s).to_string());
    }
    v
}

///
/// Writes a configuration file named `nanvixd.toml` to a directory and returns its path.
///
fn write_config_file(directory: &TempDir, contents: &str) -> String {
    let config_path: PathBuf = directory.path().join("nanvixd.toml");
    ::std::fs::write(&config_path, contents).expect("failed to write configuration file");
    config_path.to_string_lossy().into_owned()
}

#[test]
fn log_to_stdout_defaults_to_false() {
    let args = Args::parse(argv(&["--", "/bin/foo"])).expect("parse");
    assert!(!args.log_to_stdout());
}

#[test]
fn log_to_stdout_flag_sets_true() {
    let args = Args::parse(argv(&["-log-to-stdout", "--", "/bin/foo"])).expect("parse");
    assert!(args.log_to_stdout());
}

#[test]
fn log_to_stdout_composes_with_interactive_mode() {
    let args = Args::parse(argv(&["-log-to-stdout", "--", "/bin/foo", "arg1"])).expect("parse");
    assert!(args.log_to_stdout());
    assert!(args.interactive_mode());
    assert_eq!(args.program_name(), Some("/bin/foo"));
}

#[test]
fn log_to_stdout_and_log_dir_are_mutually_exclusive() {
    let err = Args::parse(argv(&[
        "-log-to-stdout",
        "-log-dir",
        "/tmp/somewhere",
        "--",
        "/bin/foo",
    ]))
    .expect_err("parse should fail when both -log-to-stdout and -log-dir are provided");
    let msg = format!("{err}");
    assert!(msg.contains(Args::OPT_LOG_TO_STDOUT), "unexpected error: {msg}");
    assert!(msg.contains(Args::OPT_LOG_DIRECTORY), "unexpected error: {msg}");
}

#[test]
fn log_dir_alone_is_accepted() {
    let args = Args::parse(argv(&["-log-dir", "/tmp/somewhere", "--", "/bin/foo"])).expect("parse");
    assert!(!args.log_to_stdout());
    assert_eq!(args.log_directory(), "/tmp/somewhere");
}

#[test]
fn parses_http_mode_with_sockaddr() {
    let args = Args::parse(argv(&["-http-addr", "127.0.0.1:9999"])).expect("parse");
    assert_eq!(args.http_sockaddr(), Some("127.0.0.1:9999"));
    assert!(!args.interactive_mode());
    assert_eq!(args.program_name(), None);
}

#[test]
fn parses_interactive_mode() {
    let args = Args::parse(argv(&["--", "/bin/foo", "arg1"])).expect("parse");
    assert_eq!(args.http_sockaddr(), None);
    assert!(args.interactive_mode());
    assert_eq!(args.program_name(), Some("/bin/foo"));
}

#[test]
fn rejects_both_http_and_interactive() {
    let res = Args::parse(argv(&["-http-addr", "127.0.0.1:9999", "--", "/bin/foo"]));
    assert!(res.is_err(), "expected error when both modes are set");
    let msg: String = format!("{:#}", res.err().unwrap());
    assert!(msg.contains("cannot use both HTTP mode"), "unexpected error: {msg}");
}

#[test]
fn rejects_neither_http_nor_interactive() {
    let res = Args::parse(argv(&[]));
    assert!(res.is_err(), "expected error when no mode is set");
    let msg: String = format!("{:#}", res.err().unwrap());
    assert!(msg.contains("must specify either HTTP mode"), "unexpected error: {msg}");
}

#[test]
fn parses_gateway_sockaddr_flag() {
    let args = Args::parse(argv(&[
        "-http-addr",
        "127.0.0.1:9999",
        "-gateway-sockaddr",
        "/tmp/test-gw.sock",
    ]))
    .expect("parse");
    assert_eq!(args.gateway_sockaddr(), Some("/tmp/test-gw.sock"));
}

#[test]
fn gateway_sockaddr_is_none_when_unset() {
    let args = Args::parse(argv(&["-http-addr", "127.0.0.1:9999"])).expect("parse");
    assert_eq!(args.gateway_sockaddr(), None);
}

#[test]
fn rejects_gateway_sockaddr_without_value() {
    let res = Args::parse(argv(&["-gateway-sockaddr"]));
    assert!(res.is_err(), "expected error when -gateway-sockaddr has no value");
    let msg: String = format!("{:#}", res.err().unwrap());
    assert!(msg.contains("missing value for: -gateway-sockaddr"), "unexpected error: {msg}");
}

#[test]
fn allow_host_builds_allowlist_filter() {
    let args = Args::parse(argv(&[
        "-allow-host-networking",
        "-allow-host",
        "1.2.3.4",
        "-allow-host",
        "10.0.0.0/8",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    let filter = args.host_filter();
    assert!(filter.permits([1, 2, 3, 4]));
    assert!(filter.permits([10, 9, 8, 7]));
    assert!(!filter.permits([8, 8, 8, 8]));
}

#[test]
fn block_host_builds_blocklist_filter() {
    let args = Args::parse(argv(&[
        "-allow-host-networking",
        "-block-host",
        "8.8.8.8",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    let filter = args.host_filter();
    assert!(!filter.permits([8, 8, 8, 8]));
    assert!(filter.permits([1, 2, 3, 4]));
}

#[test]
fn allow_and_block_host_compose_with_deny_precedence() {
    let allow_first = Args::parse(argv(&[
        "-allow-host-networking",
        "-allow-host",
        "10.0.0.0/8",
        "-block-host",
        "10.0.0.1",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    let block_first = Args::parse(argv(&[
        "-allow-host-networking",
        "-block-host",
        "10.0.0.1",
        "-allow-host",
        "10.0.0.0/8",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    for args in [&allow_first, &block_first] {
        let filter = args.host_filter();
        assert_eq!(filter.default_action(), HostAction::Deny);
        assert!(!filter.permits([10, 0, 0, 1]));
        assert!(filter.permits([10, 0, 0, 2]));
        assert!(!filter.permits([8, 8, 8, 8]));
        // DNS stays reachable for the allowlist, but never through a block rule.
        assert!(filter.permits_connection([8, 8, 8, 8], 53));
        assert!(!filter.permits_connection([10, 0, 0, 1], 53));
    }
    assert_eq!(allow_first.host_filter(), block_first.host_filter());
}

#[test]
fn rejects_invalid_block_host_entry_with_allow_host() {
    let res = Args::parse(argv(&[
        "-allow-host-networking",
        "-allow-host",
        "1.2.3.4",
        "-block-host",
        "8.8.8.8/40",
        "--",
        "/bin/foo",
    ]));
    let msg: String = format!("{:#}", res.expect_err("expected invalid-entry error"));
    assert!(msg.contains(Args::OPT_BLOCK_HOST), "unexpected error: {msg}");
    assert!(msg.contains("8.8.8.8/40"), "unexpected error: {msg}");
}

#[test]
fn host_filter_requires_networking_enabled() {
    let res = Args::parse(argv(&["-allow-host", "1.2.3.4", "--", "/bin/foo"]));
    let msg: String = format!("{:#}", res.expect_err("expected requires-networking error"));
    assert!(msg.contains(Args::OPT_ALLOW_HOST_NETWORKING), "unexpected error: {msg}");
}

#[test]
fn rejects_invalid_host_entry() {
    let res = Args::parse(argv(&[
        "-allow-host-networking",
        "-allow-host",
        "not-an-ip",
        "--",
        "/bin/foo",
    ]));
    let msg: String = format!("{:#}", res.expect_err("expected invalid-entry error"));
    assert!(msg.contains("invalid"), "unexpected error: {msg}");
}

#[test]
fn host_filter_defaults_to_allow_all() {
    let args = Args::parse(argv(&["-allow-host-networking", "--", "/bin/foo"])).expect("parse");
    assert_eq!(args.host_filter(), HostFilter::allow_all());
}

#[test]
fn bin_dir_defaults_to_executable_directory() {
    let args = Args::parse(argv(&["--", "/bin/foo"])).expect("parse");
    let executable_path: PathBuf =
        ::std::env::current_exe().expect("failed to get current executable path");
    let expected: &Path = executable_path
        .parent()
        .expect("current executable has no parent directory");
    assert_eq!(Path::new(args.binary_directory()), expected);
}

#[test]
fn bin_dir_flag_overrides_default() {
    let args =
        Args::parse(argv(&["-bin-dir", "/opt/nanvix/bin", "--", "/bin/foo"])).expect("parse");
    assert_eq!(args.binary_directory(), "/opt/nanvix/bin");
}

#[test]
fn config_file_sets_options() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(
        &tmp_dir,
        "http_addr = \"127.0.0.1:9999\"\nbin_dir = \"bin\"\nconsole_file = \
         \"console.log\"\nkernel_args = \"snapshot\"\nallow_host_networking = true\nlog_to_stdout \
         = true\n",
    );

    let args = Args::parse(argv(&["-config", &config_path])).expect("parse");
    assert_eq!(args.config_path(), Some(config_path.as_str()));
    assert_eq!(args.http_sockaddr(), Some("127.0.0.1:9999"));
    assert!(!args.interactive_mode());
    assert_eq!(Path::new(args.binary_directory()), tmp_dir.path().join("bin"));
    assert_eq!(args.console_file().map(PathBuf::from), Some(tmp_dir.path().join("console.log")));
    assert_eq!(args.kernel_args(), Some("snapshot"));
    assert!(args.networking_mode().is_enabled());
    assert!(args.log_to_stdout());
}

#[test]
fn command_line_overrides_config_file() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(
        &tmp_dir,
        "http_addr = \"127.0.0.1:9999\"\nbin_dir = \"bin\"\nkernel_args = \"snapshot\"\n",
    );

    let args = Args::parse(argv(&[
        "-config",
        &config_path,
        "-http-addr",
        "127.0.0.1:8888",
        "-bin-dir",
        "/opt/nanvix/bin",
        "-kernel-args",
        "debug",
    ]))
    .expect("parse");
    assert_eq!(args.http_sockaddr(), Some("127.0.0.1:8888"));
    assert_eq!(args.binary_directory(), "/opt/nanvix/bin");
    assert_eq!(args.kernel_args(), Some("debug"));
}

#[test]
fn interactive_mode_overrides_config_file_http_addr() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(&tmp_dir, "http_addr = \"127.0.0.1:9999\"\n");

    let args = Args::parse(argv(&["-config", &config_path, "--", "/bin/foo"])).expect("parse");
    assert!(args.interactive_mode());
    assert_eq!(args.http_sockaddr(), None);
}

#[test]
fn command_line_log_destination_overrides_config_file() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(&tmp_dir, "log_dir = \"logs\"\n");
    let args = Args::parse(argv(&["-config", &config_path, "-log-to-stdout", "--", "/bin/foo"]))
        .expect("parse");
    assert!(args.log_to_stdout());

    let config_path: String = write_config_file(&tmp_dir, "log_to_stdout = true\n");
    let args = Args::parse(argv(&[
        "-config",
        &config_path,
        "-log-dir",
        "/tmp/somewhere",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    assert!(!args.log_to_stdout());
    assert_eq!(args.log_directory(), "/tmp/somewhere");
}

#[test]
fn config_file_and_command_line_host_rules_accumulate() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(
        &tmp_dir,
        "allow_hosts = [\"10.0.0.0/8\"]\nblock_hosts = [\"10.0.0.1\"]\n",
    );

    let args = Args::parse(argv(&[
        "-config",
        &config_path,
        "-allow-host-networking",
        "-allow-host",
        "192.168.0.0/16",
        "-block-host",
        "10.0.0.2",
        "--",
        "/bin/foo",
    ]))
    .expect("parse");
    let filter = args.host_filter();
    assert!(filter.permits([10, 0, 0, 3]));
    assert!(filter.permits([192, 168, 1, 1]));
    assert!(!filter.permits([10, 0, 0, 1]));
    assert!(!filter.permits([10, 0, 0, 2]));
    assert!(!filter.permits([8, 8, 8, 8]));
}

#[test]
fn config_file_host_rules_require_networking_enabled() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    let config_path: String = write_config_file(&tmp_dir, "allow_hosts = [\"10.0.0.0/8\"]\n");

    let res = Args::parse(argv(&["-config", &config_path, "--", "/bin/foo"]));
    let msg: String = format!("{:#}", res.expect_err("expected requires-networking error"));
    assert!(msg.contains("allow_host_networking"), "unexpected error: {msg}");
}

#[test]
fn config_file_sets_cpu_mask() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temp dir");
    ::std::fs::write(tmp_dir.path().join("hwloc.json"), r#"{"nanvixd_core_str": "2-3"}"#)
        .expect("failed to write hwloc file");
    let config_path: String = write_config_file(&tmp_dir, "hwloc = \"hwloc.json\"\n");

    let args = Args::parse(argv(&["-config", &config_path, "--", "/bin/foo"])).expect("parse");
    assert_eq!(args.cpu_mask(), Some("2-3"));
}

#[test]
fn rejects_config_without_value() {
    let res = Args::parse(argv(&["-config"]));
    let msg: String = format!("{:#}", res.expect_err("expected missing-value error"));
    assert!(msg.contains("missing value for: -config"), "unexpected error: {msg}");
}
