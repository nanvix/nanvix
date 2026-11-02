// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn parse_trace_line_parses_tagged_syscall_logs() {
    // Verifies SYSCALL-tagged lines produce syscall entries with normalized modules.
    let line: &str =
        "[TRACE][SYSCALL][syscall::unistd::bindings::lseek] lseek(fd=3, offset=0, whence=0)";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::Syscall(entry)) => {
            assert_eq!(entry.module, "unistd::bindings::lseek");
            assert_eq!(entry.function, "lseek");
        },
        other => panic!("unexpected parse result: {other:?}"),
    }
}

#[test]
fn parse_trace_line_marks_libcall_logs_as_non_syscall() {
    // Ensures LIBCALL targets are classified as filtered events.
    let line: &str = "TRACE [libcall::posix::sys::stat] chmod(path=\"/tmp/demo\", mode=420)";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::NonSyscall) => {},
        other => panic!("expected non-syscall classification, got {other:?}"),
    }
}

#[test]
fn parse_trace_line_accepts_legacy_posix_target() {
    // Confirms legacy TRACE prefix without namespace tags is treated as non-syscall.
    let line: &str = "TRACE [posix::sys::stat] chmod(): path=\"/tmp/demo\", mode=420";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::NonSyscall) => {},
        other => panic!("expected non-syscall classification, got {other:?}"),
    }
}

#[test]
fn parse_trace_line_accepts_bracketed_trace_prefix() {
    // Checks bracketed TRACE prefix with libcall modules remains filtered.
    let line: &str = "[TRACE][libcall::posix::sys::stat] chmod(path=\"/tmp/demo\", mode=420)";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::NonSyscall) => {},
        other => panic!("expected non-syscall classification, got {other:?}"),
    }
}

#[test]
fn parse_trace_line_parses_syscall_logs() {
    // Validates plain syscall:: targets are parsed as syscall entries.
    let line: &str =
        "TRACE [syscall::unistd::bindings::read] read(): fd=3, buffer=0xdeadbeef, count=4";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::Syscall(entry)) => {
            assert_eq!(entry.module, "unistd::bindings::read");
            assert_eq!(entry.function, "read");
        },
        other => panic!("unexpected parse result: {other:?}"),
    }
}

#[test]
fn parse_trace_line_ignores_other_targets() {
    // Ensures unrelated targets are treated as filtered events.
    let line: &str =
        "TRACE [other::linux::fcntl] openat(): tid=42, request=OpenAtRequest { dirfd: -100 }";
    match parse_trace_line(line).expect("parse failure") {
        Some(TraceLine::NonSyscall) => {},
        other => panic!("expected non-syscall classification, got {other:?}"),
    }
}

#[test]
fn parse_trace_line_rejects_malformed_lines() {
    // Asserts malformed entries without function bodies yield errors.
    let line: &str = "TRACE [libcall::posix::sys::stat]";
    let result: Result<Option<TraceLine>> = parse_trace_line(line);
    assert!(result.is_err());
}

#[test]
fn syscall_qualified_call_strips_module_path() {
    // Confirms syscall entries drop module prefixes in `qualified_call()`.
    let entry: TraceEntry = TraceEntry {
        module: "sys::stat::bindings::fstat".to_string(),
        function: "fstat".to_string(),
    };
    assert_eq!(entry.qualified_call(), "fstat");
}

#[test]
fn extract_cmdline_from_line_finds_value() {
    // Verifies cmdline parsing when the marker is present.
    let line: &str = "[INFO][microvm] parse_bootinfo(): cmdline=\"python3 src/user/demo\"";
    let cmdline: Option<String> = extract_cmdline_from_line(line);
    assert_eq!(cmdline.as_deref(), Some("python3 src/user/demo"));
}

#[test]
fn extract_cmdline_from_line_handles_absence() {
    // Confirms absence of cmdline marker returns None.
    let line: &str = "[INFO][kernel] kmain(): initializing";
    assert!(extract_cmdline_from_line(line).is_none());
}
