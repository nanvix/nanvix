// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::{
    anyhow,
    bail,
    Context,
    Result,
};
use ::clap::Parser;
use ::std::{
    collections::HashMap,
    fs::File,
    io::{
        BufRead,
        BufReader,
    },
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// CLI Definition
//==================================================================================================

/// Command-line flags accepted by the Nanvix trace analyzer.
#[derive(Debug, Parser)]
#[command(author, version, about = "Analyze syscall trace logs", long_about = None)]
struct Cli {
    /// # Description
    /// Path to a log file containing `syscall::*` trace entries.
    #[arg(short = 't', long = "trace", value_name = "FILE")]
    trace_path: PathBuf,

    /// # Description
    /// Fail immediately when a log line cannot be parsed.
    #[arg(long = "strict", default_value_t = false)]
    strict: bool,
}

//==================================================================================================
// Data Structures
//==================================================================================================

/// Classification for parsed trace lines.
#[derive(Clone, Debug, PartialEq)]
enum TraceLine {
    Syscall(TraceEntry),
    NonSyscall,
}

/// Represents a single syscall trace entry.
#[derive(Clone, Debug, PartialEq)]
struct TraceEntry {
    module: String,
    function: String,
}

impl TraceEntry {
    ///
    /// # Description
    ///
    /// Returns a normalized identifier for the traced syscall.
    ///
    /// # Parameters
    ///
    /// - `self`: Trace entry being normalized.
    ///
    /// # Returns
    ///
    /// Fully-qualified call label for the syscall.
    ///
    fn qualified_call(&self) -> String {
        self.function.clone()
    }
}

///  Aggregated statistics collected while parsing the trace.
#[derive(Debug, Default)]
struct TraceStats {
    total_events: u64,
    skipped_lines: u64,
    filtered_events: u64,
    call_counts: HashMap<String, u64>,
    cmdline: Option<String>,
}

impl TraceStats {
    ///
    /// # Description
    ///
    /// Incorporates a single trace entry into the cumulative statistics.
    ///
    /// # Parameters
    ///
    /// - `self`: Mutable reference to the accumulator.
    /// - `entry`: Syscall invocation that should be counted.
    ///
    fn ingest(&mut self, entry: TraceEntry) {
        self.total_events += 1;
        let call_counter: &mut u64 = self.call_counts.entry(entry.qualified_call()).or_insert(0);
        *call_counter += 1;
    }

    ///
    /// # Description
    ///
    /// Returns the fully-qualified calls by frequency.
    ///
    /// # Parameters
    ///
    /// - `self`: Immutable reference to the accumulated statistics.
    ///
    /// # Returns
    ///
    /// Sorted vector of `(call, count)` pairs.
    ///
    fn top_calls(&self) -> Vec<(String, u64)> {
        Self::top_entries(&self.call_counts)
    }

    ///
    /// # Description
    ///
    /// Helper that sorts map entries by descending count and ascending key.
    ///
    /// # Parameters
    ///
    /// - `source`: Map containing call counters.
    ///
    /// # Returns
    ///
    /// Sorted vector containing every entry.
    ///
    fn top_entries(source: &HashMap<String, u64>) -> Vec<(String, u64)> {
        let mut entries: Vec<(String, u64)> = source
            .iter()
            .map(|(name, count)| (name.clone(), *count))
            .collect();
        entries.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        entries
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Program entry point.
///
/// # Parameters
///
/// - None.
///
/// # Returns
///
/// `Result` conveying whether the analyzer completed successfully.
///
/// # Errors
///
/// Propagates failures reported by `analyze_trace()` or I/O initialization.
///
fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    let stats: TraceStats = analyze_trace(&cli.trace_path, cli.strict)?;
    render_report(&stats, &cli);
    Ok(())
}

///
/// # Description
///
/// Reads the provided trace file, aggregates syscall statistics, and returns the final analysis.
///
/// # Parameters
///
/// - `trace_path`: Path to the trace log that should be parsed.
/// - `strict`: When true, parsing errors abort the entire run.
///
/// # Returns
///
/// Aggregated statistics covering every processed entry.
///
/// # Errors
///
/// Propagates I/O errors and, when strict mode is enabled, parsing failures.
fn analyze_trace(trace_path: &Path, strict: bool) -> Result<TraceStats> {
    let file: File = File::open(trace_path)
        .with_context(|| format!("Unable to open trace file: {}", trace_path.display()))?;
    let reader: BufReader<File> = BufReader::new(file);
    let mut stats: TraceStats = TraceStats::default();
    for (line_number, line_result) in reader.lines().enumerate() {
        let current_line_index: usize = line_number + 1;
        let line: String = match line_result {
            Ok(value) => value,
            Err(error) => {
                if strict {
                    return Err(error)
                        .with_context(|| format!("Failed to read line {current_line_index}"));
                }
                stats.skipped_lines += 1;
                continue;
            },
        };
        if line.trim().is_empty() {
            continue;
        }
        if stats.cmdline.is_none() {
            if let Some(cmdline) = extract_cmdline_from_line(&line) {
                stats.cmdline = Some(cmdline);
            }
        }
        match parse_trace_line(&line) {
            Ok(Some(TraceLine::Syscall(entry))) => {
                stats.ingest(entry);
            },
            Ok(Some(TraceLine::NonSyscall)) => {
                stats.filtered_events += 1;
            },
            Ok(None) => {
                continue;
            },
            Err(error) => {
                if strict {
                    return Err(error)
                        .with_context(|| format!("Failed to parse line {current_line_index}"));
                }
                stats.skipped_lines += 1;
            },
        }
    }
    Ok(stats)
}

///
/// # Description
///
/// Searches for a kernel command-line snippet embedded in a log line.
///
/// # Parameters
///
/// - `line`: Raw log entry that may contain a `cmdline="..."` field.
///
/// # Returns
///
/// Command-line string when found, or `None` if the field is absent.
fn extract_cmdline_from_line(line: &str) -> Option<String> {
    let marker: &str = "cmdline=\"";
    let start_index: usize = line.find(marker)? + marker.len();
    let remainder: &str = &line[start_index..];
    let end_offset: usize = remainder.find('\"')?;
    Some(remainder[..end_offset].to_string())
}

///
/// # Description
///
/// Parses a single log line and yields a structured trace entry when it originates from the syscall
/// namespace. Non-syscall traces are surfaced as `TraceLine::NonSyscall` so they can be counted in
/// the final report.
///
/// # Parameters
///
/// - `line`: Raw log line.
///
/// # Returns
///
/// Parsed trace classification when the line matches a supported target, or `Ok(None)` when it
/// should be ignored entirely.
///
/// # Errors
///
/// Returns an error if the line uses a malformed trace format.
fn parse_trace_line(line: &str) -> Result<Option<TraceLine>> {
    let trimmed: &str = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let remainder: &str = if let Some(rest) = trimmed.strip_prefix("[TRACE]") {
        rest.trim_start()
    } else if let Some(rest) = trimmed.strip_prefix("TRACE") {
        rest.trim_start()
    } else {
        return Ok(None);
    };
    if !remainder.starts_with('[') {
        return Ok(None);
    }

    let (first_token, mut rest) = parse_bracket_token(remainder)?;
    rest = rest.trim_start();

    let mut namespace_token: Option<&str> = None;
    let mut target_segment: &str = first_token;
    if is_namespace_token(first_token) {
        namespace_token = Some(first_token);
        if !rest.starts_with('[') {
            bail!("missing module path after namespace tag");
        }
        let (module_token, remainder_after_module) = parse_bracket_token(rest)?;
        rest = remainder_after_module.trim_start();
        target_segment = module_token;
    }

    if target_segment.is_empty() {
        bail!("missing log target segment");
    }

    let message: &str = rest.trim_start();
    if message.is_empty() {
        bail!("missing function call in trace line");
    }

    let namespace_is_syscall: bool =
        namespace_token.is_some_and(tag_is_syscall) || target_segment.starts_with("syscall::");
    if !namespace_is_syscall {
        return Ok(Some(TraceLine::NonSyscall));
    }

    let module: String = normalize_syscall_module(target_segment);
    let open_paren_index: usize = message
        .find('(')
        .ok_or_else(|| anyhow!("missing '(' in trace message"))?;
    let mut function_name: &str = &message[..open_paren_index];
    function_name = function_name.trim_end_matches(':').trim();
    if function_name.is_empty() {
        bail!("empty function name in trace line");
    }
    let entry: TraceEntry = TraceEntry {
        module,
        function: function_name.to_string(),
    };
    Ok(Some(TraceLine::Syscall(entry)))
}

///
/// # Description
///
/// Extracts the contents of a bracketed token (`[token]rest`) and returns the token plus the
/// remaining string.
///
/// # Parameters
///
/// - `input`: String slice that must start with a '[' character.
///
/// # Returns
///
/// Tuple containing the extracted token and the remainder of the string.
///
/// # Errors
///
/// Returns an error if the brackets are missing or unterminated.
fn parse_bracket_token(input: &str) -> Result<(&str, &str)> {
    if !input.starts_with('[') {
        bail!("missing '[' in trace line");
    }
    let closing_index: usize = input
        .find(']')
        .ok_or_else(|| anyhow!("unterminated log target"))?;
    let token: &str = &input[1..closing_index];
    let remainder: &str = &input[closing_index + 1..];
    Ok((token, remainder))
}

///
/// # Description
///
/// Determines whether the token represents a namespace tag (e.g., `SYSCALL`).
///
/// # Parameters
///
/// - `token`: Candidate token extracted from the log line.
///
/// # Returns
///
/// `true` when the token is a namespace identifier, otherwise `false`.
fn is_namespace_token(token: &str) -> bool {
    token.eq_ignore_ascii_case("SYSCALL") || token.eq_ignore_ascii_case("LIBCALL")
}

///
/// # Description
///
/// Indicates whether the namespace tag corresponds to the syscall domain.
///
/// # Parameters
///
/// - `token`: Namespace identifier extracted from the trace line.
///
/// # Returns
///
/// `true` when the tag is `SYSCALL`, otherwise `false`.
fn tag_is_syscall(token: &str) -> bool {
    token.eq_ignore_ascii_case("SYSCALL")
}

///
/// # Description
///
/// Removes the `syscall::` prefix so module names remain stable across instrumentation styles.
///
/// # Parameters
///
/// - `target`: Fully-qualified module name emitted by tracing instrumentation.
///
/// # Returns
///
/// Module path without the leading `syscall::` component.
fn normalize_syscall_module(target: &str) -> String {
    target
        .strip_prefix("syscall::")
        .unwrap_or(target)
        .to_string()
}

///
/// # Description
///
/// Prints a human-readable summary of the aggregated statistics.
///
/// # Parameters
///
/// - `stats`: Aggregated syscall statistics to display.
/// - `cli`: Parsed CLI options controlling presentation.
fn render_report(stats: &TraceStats, cli: &Cli) {
    println!("Trace file: {}", cli.trace_path.display());
    match stats.cmdline.as_deref() {
        Some(cmdline) => println!("Cmdline: {cmdline}"),
        None => println!("Cmdline: <not found>"),
    }
    println!("Total syscall events: {}", stats.total_events);
    println!("Filtered events (non-syscall): {}", stats.filtered_events);
    println!("Skipped lines: {}", stats.skipped_lines);
    if stats.total_events == 0 {
        println!("No syscall trace entries were found.");
        return;
    }

    let top_calls: Vec<(String, u64)> = stats.top_calls();
    if !top_calls.is_empty() {
        println!("{}", format_calls_heading(top_calls.len()));
        for (call, count) in top_calls {
            let share: f64 = (count as f64 / stats.total_events as f64) * 100.0;
            println!("  - {call:<32} {count:>6} ({share:>5.2}%)");
        }
    }
}

///
/// # Description
///
/// Formats the heading for the call distribution table.
///
/// # Parameters
///
/// - `count`: Number of entries that will actually be shown.
///
/// # Returns
///
/// Heading string describing the upcoming table section.
fn format_calls_heading(count: usize) -> String {
    format!("\nAll System Calls ({} total):", count)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
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
}
