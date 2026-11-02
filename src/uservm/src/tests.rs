// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::get_stderr_writer;
use ::anyhow::{
    Result as AnyResult,
    anyhow,
};
use ::std::{
    env,
    fs,
    fs::Metadata,
    io::Write,
    path::PathBuf,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

fn unique_log_path(suffix: &str) -> AnyResult<(String, PathBuf)> {
    let mut path: PathBuf = env::temp_dir();
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("failed to compute timestamp (error={:?})", error))?
        .as_nanos();
    let file_name: String = format!("nanvix-uservm-{suffix}-{nanos}.log");
    path.push(&file_name);
    Ok((path.to_string_lossy().into_owned(), path))
}

#[test]
fn get_stderr_writer_creates_target_file() -> AnyResult<()> {
    let (path_str, path_buf): (String, PathBuf) = unique_log_path("stderr")?;

    {
        let mut writer: Box<dyn Write + Send> = get_stderr_writer(Some(path_str.clone()))?;
        writer.write_all(b"hello stderr")?;
        writer.flush()?;
    }

    let metadata: Metadata = fs::metadata(&path_buf)?;
    assert!(metadata.is_file(), "expected log path to be a regular file");
    assert!(metadata.len() >= b"hello stderr".len() as u64, "log file is unexpectedly empty");

    fs::remove_file(path_buf).ok();
    Ok(())
}

#[test]
fn get_stderr_writer_errors_when_directory_missing() {
    let nanos: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("timestamp")
        .as_nanos();
    let mut file_path: PathBuf = env::temp_dir();
    file_path.push(format!("nanvix-uservm-missing-{nanos}"));
    file_path.push("stderr.log");

    let result: AnyResult<Box<dyn Write + Send>> =
        get_stderr_writer(Some(file_path.to_string_lossy().into_owned()));
    assert!(result.is_err(), "expected failure when parent directory does not exist");
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_recognises_canonical_prefix() {
    use super::is_windows_named_pipe;
    assert!(is_windows_named_pipe(r"\\.\pipe\foo"));
    assert!(is_windows_named_pipe(r"\\.\pipe\containerd-shim-abc"));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_recognises_long_path_prefix() {
    use super::is_windows_named_pipe;
    assert!(is_windows_named_pipe(r"\\?\pipe\foo"));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_rejects_regular_paths() {
    use super::is_windows_named_pipe;
    assert!(!is_windows_named_pipe(r"C:\temp\foo.log"));
    assert!(!is_windows_named_pipe(r"foo.log"));
    assert!(!is_windows_named_pipe(r"\\server\share\foo"));
    assert!(!is_windows_named_pipe(""));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_requires_trailing_separator() {
    use super::is_windows_named_pipe;
    // The prefix must include the trailing backslash; "\\.\pipefoo" is not a pipe path.
    assert!(!is_windows_named_pipe(r"\\.\pipefoo"));
    assert!(!is_windows_named_pipe(r"\\?\pipefoo"));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_recognises_case_variants() {
    use super::is_windows_named_pipe;
    // Windows treats pipe paths as case-insensitive: callers may legitimately
    // pass mixed-case prefixes.
    assert!(is_windows_named_pipe(r"\\.\PIPE\foo"));
    assert!(is_windows_named_pipe(r"\\.\Pipe\containerd-shim"));
    assert!(is_windows_named_pipe(r"\\?\PIPE\foo"));
    assert!(is_windows_named_pipe(r"\\?\PiPe\foo"));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_handles_short_inputs() {
    use super::is_windows_named_pipe;
    // Inputs shorter than the prefix must not panic.
    assert!(!is_windows_named_pipe(r"\\.\pipe"));
    assert!(!is_windows_named_pipe(r"\\.\"));
    assert!(!is_windows_named_pipe(r"\"));
}

#[cfg(windows)]
#[test]
fn is_windows_named_pipe_rejects_out_of_scope_variants() {
    use super::is_windows_named_pipe;
    // Remote UNC pipe paths (\\<server>\pipe\<name>) and forward-slash
    // variants are intentionally out of scope: in-tree callers always
    // supply local-machine, backslash-canonical paths. These tests pin
    // the current behavior so a future contributor extending the helper
    // to cover them also updates this test.
    assert!(!is_windows_named_pipe(r"\\myserver\pipe\foo"));
    assert!(!is_windows_named_pipe("//./pipe/foo"));
    assert!(!is_windows_named_pipe("//?/pipe/foo"));
}
