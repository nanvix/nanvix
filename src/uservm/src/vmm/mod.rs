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
    pub input: Box<StdinFn>,
    pub output: Box<StdoutFn>,
    pub stderr: Box<StderrFn>,
}

impl std::fmt::Debug for MicroVmArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MicroVmArgs")
            .field("memory_size", &self.memory_size)
            .field("kernel_filename", &self.kernel_filename)
            .field("initrd_filename", &self.initrd_filename)
            .field("initrd_args", &self.initrd_args)
            .finish()
    }
}
