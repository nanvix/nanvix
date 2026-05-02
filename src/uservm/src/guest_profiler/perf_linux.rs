// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! `perf`-based host kernel stack capture for guest flamegraphs (Linux).
//!
//! This is the Linux equivalent of the Windows ETW module (`etw.rs`).
//! It uses `perf record` to capture CPU sampling with kernel stacks
//! while the guest profiler runs, producing `perf.data` for later
//! host/guest stack merging.
//!
//! # How it works
//!
//! 1. **Before VM run**: Start `perf record` with CPU sampling and
//!    kernel+user stacks on the nanvixd process.
//! 2. **During VM run**: Our profiler records guest samples while
//!    `perf record` runs concurrently on the host.
//! 3. **After VM exit**: Stop `perf record`, convert to folded stacks
//!    with `perf script | inferno-collapse-perf`, filter by nanvixd.
//!
//! # Sampling rate alignment
//!
//! Both our profiler and `perf record` sample at ~1kHz for broadly
//! comparable analysis. Timestamp-based correlation is not implemented;
//! host and guest stacks are merged by stack identity, not by time.

use std::{
    os::unix::fs::PermissionsExt,
    process::{
        Child,
        Command,
        Stdio,
    },
};

use ::libc::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Default profiling frequency in Hz. Controls the `perf record -F` sampling
/// rate. Matches the guest profiler's default for consistent weighting.
const DEFAULT_FREQ_HZ: u64 = 1000;

/// Minimum allowed profiling frequency (Hz).
const MIN_FREQ_HZ: u64 = 1;

/// Maximum allowed profiling frequency (Hz). Values above this can cause
/// excessive overhead from perf ring buffer processing.
const MAX_FREQ_HZ: u64 = 10_000;

/// Environment variable controlling the profiling frequency (Hz).
const PROFILER_FREQ_ENV: &str = "NANVIX_PROFILER_FREQ_HZ";

/// Path to the kernel perf_event_paranoid sysctl. Values <= 0 allow
/// system-wide profiling without root.
const PERF_EVENT_PARANOID_PATH: &str = "/proc/sys/kernel/perf_event_paranoid";

/// Identifier string for perf_event file descriptors in /proc/<pid>/fd
/// symlink targets (e.g. `anon_inode:[perf_event]`).
const PERF_EVENT_FD_IDENTIFIER: &str = "perf_event";

/// Maximum time (seconds) to wait for perf to open its perf_event fds
/// before giving up on readiness detection.
const PERF_READINESS_TIMEOUT_SECS: u64 = 5;

/// File permissions applied to perf.data after recording. perf record
/// running as root creates the file with mode 0600; we relax it so the
/// merge script can read it without root.
const PERF_DATA_PERMISSIONS: u32 = 0o644;

/// Nanoseconds per second — timestamp frequency for CLOCK_MONOTONIC_RAW.
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Manages a `perf record` session for kernel stack sampling.
pub struct PerfSession {
    /// Path to the output perf.data file.
    output_path: String,
    /// Running `perf record` child process.
    child: Option<Child>,
    /// PID of the nanvixd process (for filtering).
    pid: u32,
}

impl PerfSession {
    /// Creates a new perf session that will write to the given path.
    pub fn new(output_path: &str) -> Self {
        Self {
            output_path: output_path.to_string(),
            child: None,
            pid: std::process::id(),
        }
    }

    /// Starts `perf record` with CPU sampling and kernel stacks.
    ///
    /// Uses system-wide sampling (`-a`) when running as root or with
    /// `perf_event_paranoid <= 0`, to capture kernel stacks (kvm, ioctl,
    /// etc.) that are invisible with `-p PID` (which implicitly sets
    /// `exclude_kernel=1`). When system-wide access is not available,
    /// selects per-PID mode instead (no kernel stacks, user stacks only).
    /// There is no automatic retry: the mode is chosen once based on
    /// `can_system_wide()` before spawning `perf record`.
    pub fn start(&mut self) -> Result<(), String> {
        if self.child.is_some() {
            return Err("perf session already active".to_string());
        }

        let freq_hz: u64 = std::env::var(PROFILER_FREQ_ENV)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_FREQ_HZ)
            .clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
        // Subtract 1 to avoid aliasing with the guest timer at the same rate.
        let perf_freq: String = freq_hz.saturating_sub(1).max(1).to_string();
        let pid_str: String = self.pid.to_string();

        // Try system-wide first (includes kernel stacks), fall back to per-PID.
        let (args, mode): (Vec<&str>, &str) = if Self::can_system_wide() {
            (
                vec![
                    "record",
                    "-a",
                    "-F",
                    &perf_freq,
                    "-g",
                    "--call-graph",
                    "fp",
                    "-o",
                    &self.output_path,
                ],
                "system-wide",
            )
        } else {
            (
                vec![
                    "record",
                    "-F",
                    &perf_freq,
                    "-g",
                    "--call-graph",
                    "fp",
                    "-p",
                    &pid_str,
                    "-o",
                    &self.output_path,
                ],
                "per-PID (no kernel stacks)",
            )
        };

        let child: Child = Command::new("perf")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                let msg: String = format!("Failed to start perf record: {e}");
                eprintln!("PERF_SESSION: error: {msg}");
                msg
            })?;

        self.child = Some(child);

        // Wait for perf to initialize its ring buffers before returning.
        // perf creates the output file early, but doesn't start sampling
        // until perf_event_open fds are set up. We detect readiness by
        // checking /proc/PID/fd for anon_inode:[perf_event] entries.
        // Without this, short workloads finish before perf captures samples.
        if let Some(ref mut c) = self.child {
            let perf_pid: u32 = c.id();
            let fd_path: String = format!("/proc/{perf_pid}/fd");
            let deadline: std::time::Instant = std::time::Instant::now()
                + std::time::Duration::from_secs(PERF_READINESS_TIMEOUT_SECS);
            loop {
                if std::time::Instant::now() > deadline {
                    eprintln!("PERF_SESSION: warning: timed out waiting for perf to initialize");
                    break;
                }
                // Check if perf exited early (permissions error, bad args, etc.).
                if let Ok(Some(status)) = c.try_wait() {
                    let code: i32 = status.code().unwrap_or(-1);
                    self.child = None;
                    return Err(format!(
                        "perf exited immediately (code {code}) -- check permissions or \
                         perf_event_paranoid"
                    ));
                }
                // Check if perf has opened perf_event file descriptors.
                if let Ok(entries) = std::fs::read_dir(&fd_path) {
                    let has_perf_events: bool = entries.filter_map(|e| e.ok()).any(|e| {
                        std::fs::read_link(e.path())
                            .map(|t| t.to_string_lossy().contains(PERF_EVENT_FD_IDENTIFIER))
                            .unwrap_or(false)
                    });
                    if has_perf_events {
                        // Give perf a small extra moment to finish setup.
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        eprintln!(
            "PERF_SESSION: started perf record ({mode}) -F {perf_freq} -o {}",
            self.output_path
        );
        Ok(())
    }

    /// Check if we can use system-wide perf recording (requires root or
    /// `perf_event_paranoid <= 0`).
    fn can_system_wide() -> bool {
        // Check perf_event_paranoid: -1 or 0 allows system-wide without root.
        if let Ok(val) = std::fs::read_to_string(PERF_EVENT_PARANOID_PATH)
            && let Ok(n) = val.trim().parse::<i32>()
            && n <= 0
        {
            return true;
        }
        // Check if running as root.
        unsafe { libc::geteuid() == 0 }
    }

    /// Stops the `perf record` session by sending SIGINT.
    pub fn stop(&mut self) -> Result<String, String> {
        let mut child: Child = self
            .child
            .take()
            .ok_or_else(|| "No active perf session".to_string())?;

        // Send SIGINT to perf to gracefully stop recording.
        let pid: u32 = child.id();
        let kill_ret: c_int = unsafe { libc::kill(pid.cast_signed(), libc::SIGINT) };
        if kill_ret != 0 {
            let err: std::io::Error = std::io::Error::last_os_error();
            // Still reap the child to avoid zombie processes.
            let _ = child.wait();
            return Err(format!("Failed to send SIGINT to perf (pid {pid}): {err}"));
        }

        // Wait for perf to finish writing.
        let output: std::process::Output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for perf: {e}"))?;

        if !output.status.success() {
            // perf returns non-zero on SIGINT but that's expected.
            // code() is None when killed by signal (treat as -1).
            let code: i32 = output.status.code().unwrap_or(-1);
            if code != 2 && code != -1 {
                // code 2 = interrupted, -1 = signal
                let stderr: std::borrow::Cow<'_, str> = String::from_utf8_lossy(&output.stderr);
                return Err(format!("perf record failed (code {code}): {stderr}"));
            }
        }

        // Make perf.data readable by the script's merge step (perf record
        // running as root creates the file with mode 0600).
        let _ = std::fs::set_permissions(
            &self.output_path,
            std::fs::Permissions::from_mode(PERF_DATA_PERMISSIONS),
        );

        eprintln!("PERF_SESSION: saved perf data to {}", self.output_path);
        Ok(self.output_path.clone())
    }

    /// Returns the timestamp frequency (nanoseconds for CLOCK_MONOTONIC_RAW).
    pub fn timestamp_frequency(&self) -> u64 {
        NANOS_PER_SECOND
    }
}

impl Drop for PerfSession {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            eprintln!("PERF_SESSION: killing active session on drop");
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
