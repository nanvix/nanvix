// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::nanvix_sandbox_config::{
    HostFilter,
    NetworkingMode,
};
use ::std::path::PathBuf;
use ::tempfile::TempDir;

#[tokio::test]
async fn gateway_endpoint_path_removes_socket_on_drop() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
    let path: PathBuf = tmp_dir.path().join("gateway.sock");
    let path_str: &str = path.to_str().expect("temporary path is not valid UTF-8");
    let (endpoint, gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
        bind_gateway_endpoint(path_str)
            .await
            .expect("failed to bind gateway endpoint");
    assert_eq!(gateway_path.as_str(), path_str);
    assert!(path.exists(), "gateway socket should exist once bound");

    // The socket file must go away with its path, even while the listener is still open.
    drop(gateway_path);
    assert!(!path.exists(), "gateway socket should be removed along with its path");
    drop(endpoint);
}

#[tokio::test]
async fn gateway_endpoint_path_leaves_replaced_socket_alone() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
    let path: PathBuf = tmp_dir.path().join("gateway.sock");
    let path_str: &str = path.to_str().expect("temporary path is not valid UTF-8");
    let (old_endpoint, old_gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
        bind_gateway_endpoint(path_str)
            .await
            .expect("failed to bind gateway endpoint");
    // Another daemon that is configured with the same path binds its own gateway endpoint
    // there, which replaces the socket file.
    let (new_endpoint, new_gateway_path): (GatewayEndpoint, GatewayEndpointPath) =
        bind_gateway_endpoint(path_str)
            .await
            .expect("failed to bind replacing gateway endpoint");

    drop(old_gateway_path);
    assert!(path.exists(), "replacing gateway socket should be left alone");
    drop(new_gateway_path);
    assert!(!path.exists(), "gateway socket should be removed along with its path");
    drop(old_endpoint);
    drop(new_endpoint);
}

#[tokio::test(flavor = "multi_thread")]
async fn serve_kill_removes_gateway_socket() {
    let tmp_dir: TempDir = TempDir::new().expect("failed to create temporary directory");
    let gateway_path: PathBuf = tmp_dir.path().join("gateway.sock");
    // VMs fail to boot right away, as their kernel does not exist.
    let config: StandaloneConfig = StandaloneConfig::new(
        tmp_dir
            .path()
            .join("missing-kernel.elf")
            .to_string_lossy()
            .into_owned(),
        None,
        None,
        None,
        None,
        None,
        NetworkingMode::Disabled,
        HostFilter::default(),
        #[cfg(feature = "gdb")]
        None,
        Some(gateway_path.to_string_lossy().into_owned()),
    );
    let state: Arc<StandaloneState> = Arc::new(StandaloneState::new(config));

    let new_message: message::New = message::New {
        tenant_id: "tenant".to_string(),
        app_name: "app".to_string(),
        program: tmp_dir
            .path()
            .join("program.elf")
            .to_string_lossy()
            .into_owned(),
        program_args: String::new(),
    };
    let new_response: message::NewResponse = HttpClient::serve_new(state.clone(), &new_message)
        .await
        .expect("failed to start VM");
    assert!(gateway_path.exists(), "gateway socket should exist while the VM is running");

    let kill_message: message::Kill = message::Kill {
        user_vm_id: new_response.user_vm_id,
    };
    let kill_response: message::KillResponse = HttpClient::serve_kill(state.clone(), &kill_message)
        .await
        .expect("failed to kill VM");
    assert_eq!(kill_response.exit_code, -1, "VM without a kernel should fail");
    // The reply to the KILL request is built from the returned response, so the socket must
    // be gone by the time the client learns that the VM exited.
    assert!(!gateway_path.exists(), "gateway socket should be removed when KILL completes");
    assert!(!state.has_running_vm().await, "VM should no longer be running");
}
