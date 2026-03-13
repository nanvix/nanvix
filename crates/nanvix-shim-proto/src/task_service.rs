//! NanvixTaskService — implements the containerd ttrpc Task API.
//!
//! Each method delegates to the `ExecutionMode` trait, making the task service
//! independent of any specific Nanvix execution mode.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use containerd_shim_protos::api;
use containerd_shim_protos::ttrpc;

use nanvix_shim_core::execution::ExecutionMode;

pub struct NanvixTaskService {
    mode: Arc<dyn ExecutionMode>,
}

impl NanvixTaskService {
    pub fn new(mode: Arc<dyn ExecutionMode>) -> Self {
        Self { mode }
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

        let ws = self.mode.state().await.map_err(ttrpc_err)?;
        let mut resp = api::StateResponse::new();
        resp.id = req.id;
        resp.bundle = req.exec_id;

        match ws {
            nanvix_shim_core::state::WorkloadState::Created => {
                resp.status = containerd_shim_protos::api::Status::CREATED.into();
            }
            nanvix_shim_core::state::WorkloadState::Running { pid } => {
                resp.status = containerd_shim_protos::api::Status::RUNNING.into();
                resp.pid = pid;
            }
            nanvix_shim_core::state::WorkloadState::Stopped {
                exit_code,
                exited_at,
            } => {
                resp.status = containerd_shim_protos::api::Status::STOPPED.into();
                resp.exit_status = exit_code;
                resp.exited_at = Some(to_proto_timestamp(exited_at)).into();
            }
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
        let spec = oci_spec::runtime::Spec::load(&spec_path).map_err(|e| {
            ttrpc::Error::Others(format!("failed to load OCI spec: {}", e))
        })?;

        let image_config = nanvix_oci::NanvixImageConfig::from_oci_spec(&spec).ok_or_else(|| {
            ttrpc::Error::Others("not a Nanvix image (missing com.nanvix.* annotations)".into())
        })?;

        let runtime_config = nanvix_shim_core::config::NanvixRuntimeConfig::load_or_default();

        // Resolve rootfs path. The OCI spec's root.path is relative to the bundle.
        // containerd passes rootfs mounts that we need to mount ourselves.
        let rootfs_path = {
            let root_path = match spec.root().as_ref() {
                Some(root) => {
                    let p = root.path().clone();
                    std::path::PathBuf::from(p)
                }
                None => std::path::PathBuf::from("rootfs"),
            };

            let rootfs_dir = if root_path.is_absolute() {
                root_path
            } else {
                bundle_path.join(root_path)
            };

            // Mount the rootfs if containerd provided mount info
            if !req.rootfs.is_empty() {
                std::fs::create_dir_all(&rootfs_dir).map_err(|e| {
                    ttrpc::Error::Others(format!("failed to create rootfs dir: {}", e))
                })?;

                for m in &req.rootfs {
                    log::info!(
                        "[{}] mounting rootfs: type={} source={} options={:?}",
                        req.id,
                        m.type_,
                        m.source,
                        m.options,
                    );
                    #[cfg(unix)]
                    {
                        use nix::mount::MsFlags;
                        nix::mount::mount(
                            Some(m.source.as_str()),
                            &rootfs_dir,
                            Some(m.type_.as_str()),
                            MsFlags::empty(),
                            Some(m.options.join(",").as_str()),
                        )
                        .map_err(|e| {
                            ttrpc::Error::Others(format!(
                                "failed to mount rootfs (type={} source={}): {}",
                                m.type_, m.source, e
                            ))
                        })?;
                    }
                }
            }

            rootfs_dir
        };

        let sandbox_config = nanvix_shim_core::execution::SandboxConfig {
            id: req.id.clone(),
            bundle_path: bundle_path.clone(),
            rootfs_path,
            image_config,
            runtime_config,
            stdin: std::path::PathBuf::from(&req.stdin),
            stdout: std::path::PathBuf::from(&req.stdout),
            stderr: std::path::PathBuf::from(&req.stderr),
        };

        self.mode.prepare(&sandbox_config).await.map_err(ttrpc_err)?;

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

        let pid = self.mode.start().await.map_err(ttrpc_err)?;

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

        self.mode.cleanup().await.map_err(ttrpc_err)?;

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

        self.mode.kill(req.signal).await.map_err(ttrpc_err)?;

        Ok(api::Empty::new())
    }

    async fn wait(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        req: api::WaitRequest,
    ) -> ttrpc::Result<api::WaitResponse> {
        log::info!("[{}] Task.Wait", req.id);

        let (exit_code, exited_at) = self.mode.wait().await;

        let mut resp = api::WaitResponse::new();
        resp.exit_status = exit_code;
        resp.exited_at = Some(to_proto_timestamp(exited_at)).into();
        Ok(resp)
    }

    async fn pids(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::PidsRequest,
    ) -> ttrpc::Result<api::PidsResponse> {
        Ok(api::PidsResponse::new())
    }

    async fn pause(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::PauseRequest,
    ) -> ttrpc::Result<api::Empty> {
        Err(ttrpc::Error::Others("pause not supported".into()))
    }

    async fn resume(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::ResumeRequest,
    ) -> ttrpc::Result<api::Empty> {
        Err(ttrpc::Error::Others("resume not supported".into()))
    }

    async fn exec(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::ExecProcessRequest,
    ) -> ttrpc::Result<api::Empty> {
        Err(ttrpc::Error::Others("exec not supported".into()))
    }

    async fn resize_pty(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::ResizePtyRequest,
    ) -> ttrpc::Result<api::Empty> {
        Err(ttrpc::Error::Others("resize_pty not supported".into()))
    }

    async fn close_io(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::CloseIORequest,
    ) -> ttrpc::Result<api::Empty> {
        Ok(api::Empty::new())
    }

    async fn update(
        &self,
        _ctx: &ttrpc::r#async::TtrpcContext,
        _req: api::UpdateTaskRequest,
    ) -> ttrpc::Result<api::Empty> {
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
        _req: api::StatsRequest,
    ) -> ttrpc::Result<api::StatsResponse> {
        Ok(api::StatsResponse::new())
    }
}

fn ttrpc_err(e: anyhow::Error) -> ttrpc::Error {
    ttrpc::Error::Others(format!("{:#}", e))
}

fn to_proto_timestamp(dt: chrono::DateTime<Utc>) -> protobuf::well_known_types::timestamp::Timestamp {
    let mut ts = protobuf::well_known_types::timestamp::Timestamp::new();
    ts.seconds = dt.timestamp();
    ts.nanos = dt.timestamp_subsec_nanos() as i32;
    ts
}
