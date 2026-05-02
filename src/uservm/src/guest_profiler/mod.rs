// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host-side guest profiler for Nanvix.
//!
//! Samples guest EIP/EBP at a configurable frequency by canceling the vCPU,
//! reading guest registers, and walking the frame-pointer chain through host
//! memory. After the VM exits, collected samples are resolved against ELF
//! symbol tables and written as folded stacks for flamegraph generation.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(target_os = "windows")]
pub mod etw;
mod gva;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
mod host_session_stub;
#[cfg(target_os = "linux")]
pub mod perf_linux;
mod samples;
mod symbols;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use samples::{
    DEFAULT_SAMPLE_CAPACITY,
    GuestProfiler,
    StackSample,
    timestamp_frequency,
    timestamp_now,
};
pub use symbols::SymbolResolver;

//==================================================================================================
// Platform-specific host kernel session
//==================================================================================================

// Re-export the platform's host kernel tracing session and helpers under
// common names so lib.rs can use a single code path instead of scattered
// #[cfg] blocks.

#[cfg(target_os = "windows")]
pub use etw::EtwSession as HostKernelSession;

/// On Linux, use the perf-based host kernel session.
#[cfg(target_os = "linux")]
pub use perf_linux::PerfSession as HostKernelSession;

/// On unsupported platforms, provide a no-op session stub so lib.rs compiles.
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub use host_session_stub::HostKernelSession;
