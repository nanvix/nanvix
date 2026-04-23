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
    if #[cfg(feature = "hyperlight")] {
        mod hyperlight;
        pub use hyperlight::*;
    } else if #[cfg(feature = "microvm")] {
        mod microvm;
        pub use microvm::*;
    } else {
        compile_error!("No machine feature enabled for uservm. Please enable either 'hyperlight' or 'microvm'");
    }
}

pub struct MicroVmArgs {
    pub control_rx: Receiver<VcpuControlCommand>,
    pub control_tx: Sender<VcpuControlResponse>,
    pub kernel_filename: String,
    pub initrd_filename: Option<String>,
    pub initrd_args: Option<String>,
    pub ramfs_filename: Option<String>,
    pub input: Box<StdinFn>,
    pub output: Box<StdoutFn>,
    #[cfg(feature = "hyperlight")]
    pub bulk_output: Box<BulkStdoutFn>,
    #[cfg(feature = "hyperlight")]
    pub bulk_input: Box<BulkStdinFn>,
    #[cfg(not(feature = "hyperlight"))]
    pub stderr: Box<StderrFn>,
    /// Optional file path for guest stderr redirection (hyperlight only).
    /// When set, process stderr is redirected to this file via `dup2` so that
    /// `DebugPrint` VM-exit output reaches the custom destination.
    #[cfg(feature = "hyperlight")]
    pub stderr_path: Option<String>,
    /// When true, skip kernel/initrd/ramfs loading and vCPU reset because the VM state will be
    /// restored from a snapshot.
    pub restoring_from_snapshot: bool,
    /// Optional host directory to mount on the guest (standalone mode only).
    pub mount_directory: Option<String>,
    /// Shared coalescing flag for IKC IRQ notification (microvm only).
    #[cfg(all(feature = "microvm", not(feature = "hyperlight")))]
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
            .field("ramfs_filename", &self.ramfs_filename)
            .field("mount_directory", &self.mount_directory)
            .finish()
    }
}
