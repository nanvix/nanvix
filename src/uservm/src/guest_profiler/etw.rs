// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! ETW-based host kernel stack correlation for guest flamegraphs.
//!
//! This module provides infrastructure for correlating Windows ETW
//! (Event Tracing for Windows) kernel stack traces with guest profiler
//! samples by timestamp and thread ID.
//!
//! # How correlation works
//!
//! Both our profiler and ETW use QPC (QueryPerformanceCounter) as their
//! time source. Each guest/host sample records a QPC timestamp at capture
//! time. After VM exit, the ETL is processed with `xperf` to extract
//! CPU sampling stacks, and each ETW sample also has a QPC timestamp.
//!
//! Correlation matches ETW kernel stacks to our samples by:
//! 1. Filtering ETW samples to the vCPU thread ID
//! 2. For each ETW sample, finding the nearest profiler sample within
//!    a configurable QPC tolerance window
//! 3. Merging the kernel stack frames below the host/guest frames
//!
//! # Sampling rate alignment
//!
//! Our profiler timer runs at 1kHz (1ms period). WPR's built-in `CPU`
//! profile also samples at ~1kHz by default. Since both are independent
//! periodic timers, their samples won't align exactly. The correlation
//! uses a tolerance window (default ±500µs) to match nearby samples.
//!
//! For tighter alignment, the QPC timestamps enable sub-microsecond
//! matching precision regardless of sampling rate differences.

use std::process::Command;

//==================================================================================================
// Constants
//==================================================================================================

/// Default profiling frequency in Hz. Controls both the guest profiler timer
/// and the ETW CPU sampling interval (via xperf -SetProfInt).
const DEFAULT_FREQ_HZ: u64 = 1000;

/// Minimum allowed profiling frequency (Hz).
const MIN_FREQ_HZ: u64 = 1;

/// Maximum allowed profiling frequency (Hz). Values above this can cause
/// excessive overhead from signal delivery and register reads.
const MAX_FREQ_HZ: u64 = 10_000;

/// Number of 100-nanosecond intervals per second. Used to convert the
/// profiling frequency (Hz) to the interval unit that `xperf -SetProfInt`
/// expects.
const HUNDRED_NS_PER_SECOND: u64 = 10_000_000;

/// Default path to xperf.exe (Windows Performance Toolkit).
/// Overridable via the `NANVIX_XPERF_PATH` environment variable.
const DEFAULT_XPERF_PATH: &str =
    "C:\\Program Files (x86)\\Windows Kits\\10\\Windows Performance Toolkit\\xperf.exe";

/// Default WPR profile name used when a custom .wprp file provides a
/// NanvixBench profile with larger buffers and Hyper-V providers.
const DEFAULT_WPR_PROFILE_NAME: &str = "NanvixBench";

/// Fallback WPR profile when no custom .wprp file is found.
const FALLBACK_WPR_PROFILE: &str = "CPU";

//==================================================================================================
// ETW Session
//==================================================================================================

/// Manages a WPR (Windows Performance Recorder) trace session.
pub struct EtwSession {
    /// Path to the output ETL file.
    output_path: String,
    /// Whether a trace is currently running.
    active: bool,
    /// QPC frequency (ticks per second) for timestamp conversion.
    qpc_frequency: u64,
}

impl EtwSession {
    /// Creates a new ETW session that will write to the given path.
    pub fn new(output_path: &str) -> Self {
        Self {
            output_path: output_path.to_string(),
            active: false,
            qpc_frequency: super::timestamp_frequency(),
        }
    }

    /// Starts a WPR trace session with CPU sampling and kernel stacks.
    ///
    /// Uses the built-in `CPU` profile which captures:
    /// - CPU sampling at ~1kHz (matching our guest profiler frequency)
    /// - Kernel + user stacks on CPU samples
    /// - Thread scheduling events
    pub fn start(&mut self) -> Result<(), String> {
        if self.active {
            let msg = "ETW session already active".to_string();
            eprintln!("ETW_SESSION: error: {msg}");
            return Err(msg);
        }

        // Cancel any lingering WPR session from a previous crash.
        let _ = Command::new("wpr").args(["-cancel"]).output();

        // Set the ETW CPU sampling interval to match the guest profiler
        // frequency. NANVIX_PROFILER_FREQ_HZ controls both.
        // xperf -SetProfInt takes 100ns units: 1kHz = 10000.
        let freq_hz: u64 = std::env::var("NANVIX_PROFILER_FREQ_HZ")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_FREQ_HZ)
            .clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
        let prof_interval_100ns = HUNDRED_NS_PER_SECOND / freq_hz;
        let xperf_path =
            std::env::var("NANVIX_XPERF_PATH").unwrap_or_else(|_| DEFAULT_XPERF_PATH.to_string());
        if std::path::Path::new(&xperf_path).exists() {
            let _ = Command::new(&xperf_path)
                .args(["-SetProfInt", &prof_interval_100ns.to_string()])
                .output();
        }

        // Use the nanvix benchmark WPR profile if available (larger buffers,
        // Hyper-V providers). Fall back to the built-in CPU profile.
        let wpr_profile = std::env::var("NANVIX_WPR_PROFILE").ok();
        let (args, profile_name) = if let Some(ref profile_path) = wpr_profile {
            let profile_arg = format!("{}!{}", profile_path, DEFAULT_WPR_PROFILE_NAME);
            (
                vec!["-start".to_string(), profile_arg, "-filemode".to_string()],
                DEFAULT_WPR_PROFILE_NAME,
            )
        } else {
            // Check for the profile in the default location relative to the exe.
            let default_profile = std::env::current_exe()
                .ok()
                .and_then(|p| {
                    p.parent()
                        .map(|d| d.join("..\\scripts\\bench\\wpr-profile.wprp"))
                })
                .filter(|p| p.exists())
                .map(|p| format!("{}!{}", p.display(), DEFAULT_WPR_PROFILE_NAME));

            if let Some(profile_arg) = default_profile {
                (
                    vec!["-start".to_string(), profile_arg, "-filemode".to_string()],
                    DEFAULT_WPR_PROFILE_NAME,
                )
            } else {
                (
                    vec![
                        "-start".to_string(),
                        FALLBACK_WPR_PROFILE.to_string(),
                        "-filemode".to_string(),
                    ],
                    FALLBACK_WPR_PROFILE,
                )
            }
        };

        let result = Command::new("wpr").args(&args).output().map_err(|e| {
            let msg = format!("Failed to start WPR: {e}");
            eprintln!("ETW_SESSION: error: {msg}");
            msg
        })?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            let msg = format!("WPR start failed: {stderr}");
            eprintln!("ETW_SESSION: error: {msg}");
            return Err(msg);
        }

        self.active = true;
        eprintln!(
            "ETW_SESSION: started WPR {} profiling (qpc_freq={})",
            profile_name, self.qpc_frequency
        );
        Ok(())
    }

    /// Stops the WPR trace and saves the ETL file.
    pub fn stop(&mut self) -> Result<String, String> {
        if !self.active {
            return Err("No active ETW session".to_string());
        }

        let result = Command::new("wpr")
            .args(["-stop", &self.output_path])
            .output()
            .map_err(|e| format!("Failed to stop WPR: {e}"))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            // Don't clear active — Drop will call wpr -cancel as cleanup.
            return Err(format!("WPR stop failed: {stderr}"));
        }

        // Only clear active after a successful stop so Drop doesn't skip
        // cancel if stop failed.
        self.active = false;

        eprintln!("ETW_SESSION: saved ETL to {}", self.output_path);
        Ok(self.output_path.clone())
    }

    /// Returns the QPC frequency for timestamp conversion.
    pub fn qpc_frequency(&self) -> u64 {
        self.qpc_frequency
    }
}

impl Drop for EtwSession {
    fn drop(&mut self) {
        if self.active {
            eprintln!("ETW_SESSION: canceling active session on drop");
            let _ = Command::new("wpr").args(["-cancel"]).output();
            self.active = false;
        }
    }
}
