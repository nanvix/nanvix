// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! NanvixSandboxService — implements the containerd ttrpc Sandbox API.
//!
//! The Sandbox API (containerd 2.0+) manages sandbox-level lifecycle.
//! For Nanvix V1, each container gets its own sandbox (no pod sharing).

use std::sync::Arc;

use async_trait::async_trait;
use containerd_shim_protos::{
    sandbox_api,
    sandbox_async,
    ttrpc,
};

use nanvix_shim_core::runtime::WorkloadRuntime;

pub struct NanvixSandboxService {
    runtime: Arc<dyn WorkloadRuntime>,
}

impl NanvixSandboxService {
    pub fn new(runtime: Arc<dyn WorkloadRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl sandbox_async::Sandbox for NanvixSandboxService {
    async fn create_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::CreateSandboxRequest,
    ) -> ttrpc::Result<sandbox_api::CreateSandboxResponse> {
        // TODO: here we will create the System VM or the system process
        log::info!("[{}] Sandbox.CreateSandbox", req.sandbox_id);
        Ok(sandbox_api::CreateSandboxResponse::new())
    }

    async fn start_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::StartSandboxRequest,
    ) -> ttrpc::Result<sandbox_api::StartSandboxResponse> {
        // TODO: here we will start the System VM or the system process
        log::info!("[{}] Sandbox.StartSandbox", req.sandbox_id);
        Ok(sandbox_api::StartSandboxResponse::new())
    }

    async fn platform(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: sandbox_api::PlatformRequest,
    ) -> ttrpc::Result<sandbox_api::PlatformResponse> {
        let mut resp = sandbox_api::PlatformResponse::new();
        let mut platform = containerd_shim_protos::types::platform::Platform::new();
        platform.os = "nanvix".to_string();
        platform.architecture = "x86".to_string();
        resp.platform = Some(platform).into();
        Ok(resp)
    }

    async fn stop_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::StopSandboxRequest,
    ) -> ttrpc::Result<sandbox_api::StopSandboxResponse> {
        // This will finally delete the System VM or the system process, but we will kill it first to be sure
        log::info!("[{}] Sandbox.StopSandbox", req.sandbox_id);
        let _ = self.runtime.kill(9).await;
        Ok(sandbox_api::StopSandboxResponse::new())
    }

    async fn wait_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::WaitSandboxRequest,
    ) -> ttrpc::Result<sandbox_api::WaitSandboxResponse> {
        log::info!("[{}] Sandbox.WaitSandbox", req.sandbox_id);
        let (exit_code, _) = self.runtime.wait().await;
        let mut resp = sandbox_api::WaitSandboxResponse::new();
        resp.exit_status = exit_code;
        Ok(resp)
    }

    async fn sandbox_status(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::SandboxStatusRequest,
    ) -> ttrpc::Result<sandbox_api::SandboxStatusResponse> {
        log::info!("[{}] Sandbox.SandboxStatus", req.sandbox_id);
        let mut resp = sandbox_api::SandboxStatusResponse::new();
        resp.sandbox_id = req.sandbox_id;
        resp.state = "running".to_string();
        Ok(resp)
    }

    async fn ping_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: sandbox_api::PingRequest,
    ) -> ttrpc::Result<sandbox_api::PingResponse> {
        Ok(sandbox_api::PingResponse::new())
    }

    async fn shutdown_sandbox(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: sandbox_api::ShutdownSandboxRequest,
    ) -> ttrpc::Result<sandbox_api::ShutdownSandboxResponse> {
        log::info!("[{}] Sandbox.ShutdownSandbox", req.sandbox_id);
        let _ = self.runtime.cleanup().await;
        Ok(sandbox_api::ShutdownSandboxResponse::new())
    }
}
