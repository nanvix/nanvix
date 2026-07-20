// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! NanvixTaskService — implements the containerd ttrpc Task API.
//!
//! Each method delegates to the workload runtime.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use containerd_shim_protos::{
    api,
    ttrpc,
};

use nanvix_shim_core::runtime::WorkloadRuntime;

pub struct NanvixTaskService {
    runtime: Arc<dyn WorkloadRuntime>,
}

impl NanvixTaskService {
    pub fn new(runtime: Arc<dyn WorkloadRuntime>) -> Self {
        Self { runtime }
    }
}

#[async_trait]
impl containerd_shim_protos::shim_async::Task for NanvixTaskService {
    async fn state(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::StateRequest,
    ) -> ttrpc::Result<api::StateResponse> {
        log::info!("[{}] Task.State", req.id);

        let ws = self.runtime.state().await.map_err(ttrpc_err)?;
        let mut resp = api::StateResponse::new();
        resp.id = req.id;
        resp.bundle = req.exec_id;

        match ws {
            nanvix_shim_core::state::WorkloadState::Created => {
                resp.status = containerd_shim_protos::api::Status::CREATED.into();
            },
            nanvix_shim_core::state::WorkloadState::Running { pid } => {
                resp.status = containerd_shim_protos::api::Status::RUNNING.into();
                resp.pid = pid;
            },
            nanvix_shim_core::state::WorkloadState::Stopped {
                exit_code,
                exited_at,
            } => {
                resp.status = containerd_shim_protos::api::Status::STOPPED.into();
                resp.exit_status = exit_code;
                resp.exited_at = Some(to_proto_timestamp(exited_at)).into();
            },
        }

        Ok(resp)
    }

    async fn create(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::CreateTaskRequest,
    ) -> ttrpc::Result<api::CreateTaskResponse> {
        log::info!("[{}] Task.Create bundle={}", req.id, req.bundle);

        // Parse OCI spec from bundle
        let bundle_path = std::path::PathBuf::from(&req.bundle);
        let spec_path = bundle_path.join("config.json");
        let spec = oci_spec::runtime::Spec::load(&spec_path)
            .map_err(|e| ttrpc::Error::Others(format!("failed to load OCI spec: {}", e)))?;

        let image_config =
            nanvix_oci::NanvixImageConfig::from_oci_spec(&spec).ok_or_else(|| {
                ttrpc::Error::Others("not a Nanvix image (missing com.nanvix.* annotations)".into())
            })?;

        let runtime_config = nanvix_shim_core::config::NanvixRuntimeConfig::load_or_default();

        // Resolve the rootfs path from the OCI spec.
        //
        // containerd unpacks image layers into the bundle's rootfs directory before
        // calling Create. The OCI spec's `root.path` field points to this directory
        // (relative to the bundle, or absolute).
        //
        // containerd also passes `req.rootfs` with mount instructions (typically
        // overlayfs combining all image layers). Both the resolved path and the
        // Mount instructions are forwarded through SandboxConfig so the standalone runtime can
        // mount the overlayfs and pass initrd and ramfs files to the user VM.
        let rootfs_path: std::path::PathBuf = {
            let root_path: std::path::PathBuf = match spec.root().as_ref() {
                Some(root) => root.path().clone(),
                None => std::path::PathBuf::from("rootfs"),
            };

            if root_path.is_absolute() {
                root_path
            } else {
                bundle_path.join(root_path)
            }
        };

        // Convert containerd mount information for the runtime.
        let rootfs_mounts: Vec<(String, String, Vec<String>)> = req
            .rootfs
            .iter()
            .map(|m| (m.type_.clone(), m.source.clone(), m.options.clone()))
            .collect();

        let sandbox_config = nanvix_shim_core::runtime::SandboxConfig {
            id: req.id.clone(),
            bundle_path: bundle_path.clone(),
            rootfs_path,
            image_config,
            runtime_config,
            stdin: std::path::PathBuf::from(&req.stdin),
            stdout: std::path::PathBuf::from(&req.stdout),
            stderr: std::path::PathBuf::from(&req.stderr),
            rootfs_mounts,
        };

        self.runtime
            .prepare(&sandbox_config)
            .await
            .map_err(ttrpc_err)?;

        let mut resp = api::CreateTaskResponse::new();
        resp.pid = 0; // not started yet
        Ok(resp)
    }

    async fn start(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::StartRequest,
    ) -> ttrpc::Result<api::StartResponse> {
        log::info!("[{}] Task.Start", req.id);

        let pid = self.runtime.start().await.map_err(ttrpc_err)?;

        let mut resp = api::StartResponse::new();
        resp.pid = pid;
        Ok(resp)
    }

    async fn delete(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::DeleteRequest,
    ) -> ttrpc::Result<api::DeleteResponse> {
        log::info!("[{}] Task.Delete", req.id);

        self.runtime.cleanup().await.map_err(ttrpc_err)?;

        let mut resp = api::DeleteResponse::new();
        resp.exit_status = 0;
        resp.exited_at = Some(to_proto_timestamp(Utc::now())).into();
        Ok(resp)
    }

    async fn kill(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::KillRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::info!("[{}] Task.Kill signal={}", req.id, req.signal);

        self.runtime.kill(req.signal).await.map_err(ttrpc_err)?;

        Ok(api::Empty::new())
    }

    async fn wait(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::WaitRequest,
    ) -> ttrpc::Result<api::WaitResponse> {
        log::info!("[{}] Task.Wait", req.id);

        let (exit_code, exited_at) = self.runtime.wait().await;

        let mut resp = api::WaitResponse::new();
        resp.exit_status = exit_code;
        resp.exited_at = Some(to_proto_timestamp(exited_at)).into();
        Ok(resp)
    }

    async fn pids(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::PidsRequest,
    ) -> ttrpc::Result<api::PidsResponse> {
        log::warn!("[{}] Task.Pids not implemented, returning empty", req.id);
        Ok(api::PidsResponse::new())
    }

    async fn pause(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::PauseRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::warn!("[{}] Task.Pause not supported", req.id);
        Err(ttrpc::Error::Others("pause not supported".into()))
    }

    async fn resume(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::ResumeRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::warn!("[{}] Task.Resume not supported", req.id);
        Err(ttrpc::Error::Others("resume not supported".into()))
    }

    async fn exec(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::ExecProcessRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::warn!("[{}] Task.Exec not supported", req.id);
        Err(ttrpc::Error::Others("exec not supported".into()))
    }

    async fn resize_pty(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::ResizePtyRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::warn!("[{}] Task.ResizePty not supported", req.id);
        Err(ttrpc::Error::Others("resize_pty not supported".into()))
    }

    async fn close_io(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::CloseIORequest,
    ) -> ttrpc::Result<api::Empty> {
        log::debug!("[{}] Task.CloseIO (no-op)", req.id);
        Ok(api::Empty::new())
    }

    async fn update(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::UpdateTaskRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::debug!("[{}] Task.Update (no-op)", req.id);
        Ok(api::Empty::new())
    }

    async fn connect(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::ConnectRequest,
    ) -> ttrpc::Result<api::ConnectResponse> {
        let mut resp = api::ConnectResponse::new();
        resp.version = "nanvix-shim-v0.1.0".to_string();
        resp.shim_pid = std::process::id();
        Ok(resp)
    }

    async fn shutdown(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::ShutdownRequest,
    ) -> ttrpc::Result<api::Empty> {
        log::info!("Task.Shutdown");
        Ok(api::Empty::new())
    }

    async fn stats(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::StatsRequest,
    ) -> ttrpc::Result<api::StatsResponse> {
        log::debug!("[{}] Task.Stats not implemented, returning empty", req.id);
        Ok(api::StatsResponse::new())
    }
}

fn ttrpc_err(e: anyhow::Error) -> ttrpc::Error {
    ttrpc::Error::Others(format!("{:#}", e))
}

fn to_proto_timestamp(
    dt: chrono::DateTime<Utc>,
) -> protobuf::well_known_types::timestamp::Timestamp {
    let mut ts = protobuf::well_known_types::timestamp::Timestamp::new();
    ts.seconds = dt.timestamp();
    ts.nanos = dt.timestamp_subsec_nanos() as i32;
    ts
}
