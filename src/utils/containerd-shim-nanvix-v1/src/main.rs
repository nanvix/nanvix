// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
use std::sync::Arc;

use nanvix_shim_core::{
    runtime::WorkloadRuntime,
    NanvixRuntimeConfig,
};
use nanvix_shim_proto::args::{
    parse_args,
    Action,
};

fn build_runtime(id: &str) -> Arc<dyn WorkloadRuntime> {
    let config: NanvixRuntimeConfig = NanvixRuntimeConfig::load_or_default();
    Arc::new(nanvix_shim_standalone::StandaloneRuntime::new(id.to_string(), config))
}

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let action = match parse_args(&args) {
        Ok(action) => action,
        Err(e) => {
            eprintln!("containerd-shim-nanvix-v1: {e:?}");
            std::process::exit(1);
        },
    };

    match action {
        Action::Start(shim_args) => {
            let runtime: Arc<dyn WorkloadRuntime> = build_runtime(&shim_args.id);
            let mut executor = nanvix_shim_proto::executor::ShimExecutor::new(shim_args, runtime);
            if let Err(e) = executor.start() {
                eprintln!("start failed: {e:?}");
                std::process::exit(1);
            }
        },
        Action::Delete(shim_args) => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let runtime: Arc<dyn WorkloadRuntime> = build_runtime(&shim_args.id);
            let mut executor = nanvix_shim_proto::executor::ShimExecutor::new(shim_args, runtime);
            if let Err(e) = rt.block_on(executor.delete()) {
                eprintln!("delete failed: {e:?}");
                std::process::exit(1);
            }
        },
        Action::Run(shim_args) => {
            let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
            let runtime: Arc<dyn WorkloadRuntime> = build_runtime(&shim_args.id);
            let mut executor = nanvix_shim_proto::executor::ShimExecutor::new(shim_args, runtime);
            if let Err(e) = rt.block_on(executor.run()) {
                eprintln!("run failed: {e:?}");
                std::process::exit(1);
            }
        },
        Action::Version => {
            println!("containerd-shim-nanvix-v1 v{}", env!("CARGO_PKG_VERSION"));
        },
        Action::Help => {
            println!("Usage: containerd-shim-nanvix-v1 [flags] [start|delete]");
            println!("  -namespace  Namespace that owns the shim");
            println!("  -id         Id of the task");
            println!("  -address    GRPC address back to containerd");
            println!("  -socket     Socket path to serve");
            println!("  -bundle     Path to the bundle");
            println!("  -debug      Enable debug output");
            println!("  -version    Show version");
        },
    }
}
