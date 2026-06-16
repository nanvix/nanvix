// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! No-op host kernel session stub for platforms without ETW or perf support.
//!
//! This allows lib.rs to use a single code path via the `HostKernelSession`
//! type alias without platform-specific `#[cfg]` blocks.

pub struct HostKernelSession;

impl HostKernelSession {
    pub fn new(_output_path: &str) -> Self {
        Self
    }

    pub fn start(&mut self) -> Result<(), String> {
        Err("Host kernel tracing not supported on this platform".to_string())
    }

    pub fn stop(&mut self) -> Result<String, String> {
        Err("No active session".to_string())
    }
}
