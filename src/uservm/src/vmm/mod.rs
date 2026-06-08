// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::orchestrator::{
    VcpuControlCommand,
    VcpuControlResponse,
};
use ::tokio::sync::mpsc::{
    Receiver,
    Sender,
};

#[cfg(feature = "profile-time")]
use crate::perf::PerfTimings;

//==================================================================================================
// Modules
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "microvm")] {
        mod microvm;
        pub use microvm::*;
    } else {
        compile_error!("No machine feature enabled for uservm. Please enable 'microvm'");
    }
}

pub struct MicroVmArgs {
    pub control_rx: Receiver<VcpuControlCommand>,
    pub control_tx: Sender<VcpuControlResponse>,
    pub kernel_filename: String,
    pub initrd_filename: Option<String>,
    pub initrd_args: Option<String>,
    pub kernel_args: Option<String>,
    pub ramfs_filename: Option<String>,
    pub input: Box<StdinFn>,
    pub output: Box<StdoutFn>,
    pub stderr: Box<StderrFn>,
    /// When true, skip kernel/initrd/ramfs loading and vCPU reset because the VM state will be
    /// restored from a snapshot.
    pub restoring_from_snapshot: bool,
    /// Shared coalescing flag for IKC IRQ notification (microvm only).
    pub ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Optional GDB server port (standalone mode only, microvm only).
    #[cfg(feature = "gdb")]
    pub gdb_port: Option<u16>,
    /// Performance timings collector for fine-grained startup breakdown.
    #[cfg(feature = "profile-time")]
    pub perf_timings: PerfTimings,
}

impl std::fmt::Debug for MicroVmArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicroVmArgs")
            .field("kernel_filename", &self.kernel_filename)
            .field("initrd_filename", &self.initrd_filename)
            .field("initrd_args", &self.initrd_args)
            .field("kernel_args", &self.kernel_args)
            .field("ramfs_filename", &self.ramfs_filename)
            .finish()
    }
}
