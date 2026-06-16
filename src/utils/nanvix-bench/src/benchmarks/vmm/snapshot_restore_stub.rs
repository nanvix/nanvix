// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Stub implementation of the snapshot-restore benchmark for builds without the
//! `profile-time` feature.
//!
//! The full implementation in [`snapshot_restore`](super::snapshot_restore) depends on the VMM's
//! per-phase performance counters, which are only compiled in when `profile-time` is enabled.
//! Rather than failing the entire `nanvix-bench` crate to build when the feature is off, this
//! stub keeps the other benchmarks usable and surfaces a runtime error if someone invokes
//! `snapshot-restore` without the required feature.

use crate::benchmark::Benchmark;
use ::anyhow::Result;

impl Benchmark {
    pub async fn run_snapshot_restore(&mut self) -> Result<()> {
        anyhow::bail!(
            "the snapshot-restore benchmark requires the `profile-time` feature to be enabled at \
             compile time"
        )
    }
}
