//! Workload state machine.

use chrono::{DateTime, Utc};

/// Represents the current state of a Nanvix workload.
#[derive(Debug, Clone)]
pub enum WorkloadState {
    /// The workload has been created but not yet started.
    Created,
    /// The workload is running. Contains the process/task ID.
    Running { pid: u32 },
    /// The workload has exited.
    Stopped {
        exit_code: u32,
        exited_at: DateTime<Utc>,
    },
}
