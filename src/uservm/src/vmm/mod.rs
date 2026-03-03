// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use tokio::sync::mpsc::{
    Receiver,
    Sender,
};

use crate::orchestrator::{
    VcpuControlCommand,
    VcpuControlResponse,
};

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
    pub memory_size: usize,
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
    pub stderr: Box<StderrFn>,
    /// When true, skip kernel/initrd/ramfs loading and vCPU reset because the VM state will be
    /// restored from a snapshot.
    pub restoring_from_snapshot: bool,
    /// Shared coalescing flag for IKC IRQ notification (microvm only).
    #[cfg(feature = "microvm")]
    pub ikc_pending: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for MicroVmArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicroVmArgs")
            .field("memory_size", &self.memory_size)
            .field("kernel_filename", &self.kernel_filename)
            .field("initrd_filename", &self.initrd_filename)
            .field("initrd_args", &self.initrd_args)
            .field("ramfs_filename", &self.ramfs_filename)
            .finish()
    }
}
