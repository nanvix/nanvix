// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    cache::{
        LockedSandbox,
        SandboxCache,
        SandboxHandle,
    },
    config,
    sandbox::{
        SandboxConfig,
        SandboxTag,
    },
};
use ::anyhow::Result;
use ::http_body_util::{
    BodyExt,
    Full,
};
use ::hyper::{
    body::{
        Bytes,
        Incoming,
    },
    service::Service,
    Request,
    Response,
    StatusCode,
};
use ::serde::Deserialize;
use ::serde_json::Value;
use ::std::{
    self,
    future::Future,
    pin::Pin,
    sync::{
        atomic::AtomicUsize,
        Arc,
    },
};
use ::tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::{
        UnixListener,
        UnixStream,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Deserialize)]
struct MessageJson {
    clientid: usize,
    program: String,
    args: Vec<String>,
}

pub struct HttpClient {
    sandboxes: SandboxCache,
    requestid: Arc<AtomicUsize>,
    linuxd_listener: Arc<UnixListener>,
    linuxd_sockaddr: String,
    sandbox_sockaddr: String,
    console_file: String,
}

impl HttpClient {
    pub fn new(
        sandboxes: SandboxCache,
        requestid: Arc<AtomicUsize>,
        linuxd_listener: Arc<UnixListener>,
        linuxd_sockaddr: String,
        sandbox_sockaddr: String,
        console_file: String,
    ) -> Self {
        Self {
            sandboxes,
            requestid,
            linuxd_listener,
            linuxd_sockaddr,
            sandbox_sockaddr,
            console_file,
        }
    }

    ///
    /// # Description
    ///
    /// Helper function that creates a "bad request" response.
    ///
    /// # Returns
    ///
    /// A "bad request" response.
    ///
    fn bad_request() -> Response<Full<Bytes>> {
        let mut bad_request: Response<Full<Bytes>> = Response::new(Full::new(Bytes::new()));
        *bad_request.status_mut() = hyper::StatusCode::BAD_REQUEST;
        bad_request
    }

    ///
    /// # Description
    ///
    /// Helper function that creates an "internal server error" response.
    ///
    /// # Returns
    ///
    /// An "internal server error" response.
    ///
    fn internal_server_error() -> Response<Full<Bytes>> {
        let mut internal_server_error: Response<Full<Bytes>> =
            Response::new(Full::new(Bytes::new()));
        *internal_server_error.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        internal_server_error
    }

    async fn serve(
        sandbox_cache: &SandboxCache,
        request: MessageJson,
        requestid: usize,
        linuxd_sockaddr: String,
        console_file: String,
        sandbox_sockaddr: String,
        linuxd_listener: Arc<UnixListener>,
    ) -> Result<Vec<u8>> {
        let linuxd_sockaddr: String =
            config::linuxd_sockaddr_builder(&linuxd_sockaddr, request.clientid, requestid);

        let tag: SandboxTag = SandboxTag::new(request.clientid, &request.program);
        let config: SandboxConfig =
            SandboxConfig::new(linuxd_listener, &linuxd_sockaddr, &sandbox_sockaddr, &console_file);
        let mut sandbox: SandboxHandle = sandbox_cache.get(&tag, &config).await?;

        let mut locked_sandbox: LockedSandbox = sandbox.get_sandbox().await?;

        let sandbox_socket: &mut UnixStream = locked_sandbox.socket()?;

        let buf: Vec<u8> = request.args.join(" ").as_bytes().to_vec();
        if buf.len() > config::MAX_PAYLOAD_SIZE {
            let reason: String = format!(
                "payload size exceeds maximum protocol limit (size={}, limit={})",
                buf.len(),
                config::MAX_PAYLOAD_SIZE
            );
            error!("serve(): {}", reason);
            anyhow::bail!(reason)
        }

        // Forward request length to Sandbox.
        let length = buf.len() as u8;
        if let Err(e) = sandbox_socket.write_all(&[length]).await {
            let reason: String = format!("failed to write length byte to sandbox (error={:?})", e);
            error!("serve(): {}", reason);
            anyhow::bail!(reason)
        }

        // Forward request to Sandbox.
        if let Err(e) = sandbox_socket.write_all(&buf).await {
            let reason: String = format!("failed to write bytes to sandbox (error={:?})", e);
            error!("serve(): {}", reason);
            anyhow::bail!(reason)
        }

        // Read the length byte from Sandbox.
        let mut length_byte: [u8; 1] = [0u8; 1];
        if let Err(e) = sandbox_socket.read_exact(&mut length_byte).await {
            let reason: String = format!("failed to read length byte from sandbox (error={:?})", e);
            error!("serve(): {}", reason);
            anyhow::bail!(reason)
        }

        // Read the actual data bytes from Sandbox.
        let data_length: usize = length_byte[0] as usize;
        let mut bytes: Vec<u8> = vec![0u8; data_length];
        if let Err(e) = sandbox_socket.read_exact(&mut bytes).await {
            let reason: String = format!("failed to read data bytes from sandbox (error={:?})", e);
            error!("serve(): {}", reason);
            anyhow::bail!(reason)
        }

        Ok(bytes)
    }
}

impl Service<Request<Incoming>> for HttpClient {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let requestid = self
            .requestid
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let sandbox_sockaddr: String = self.sandbox_sockaddr.clone();
        let linuxd_sockaddr: String = self.linuxd_sockaddr.clone();
        let nanvix_console: String = self.console_file.clone();
        let linuxd_listener: Arc<UnixListener> = self.linuxd_listener.clone();
        let sandboxes: SandboxCache = self.sandboxes.clone();
        let future = async move {
            let body: Bytes = match request.collect().await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let reason: String = "failed to read body".to_string();
                    error!("{}", reason);
                    return Ok(Self::internal_server_error());
                },
            };

            // Deserialize the JSON directly into the struct
            let request: MessageJson = match serde_json::from_slice(body.as_ref()) {
                Ok(request) => request,
                Err(_) => {
                    let reason: String = "failed to deserialize JSON".to_string();
                    error!("{}", reason);
                    return Ok(Self::bad_request());
                },
            };

            // Print out the deserialized struct
            debug!("request id: {}", requestid);
            debug!("client id: {}", request.clientid);
            debug!("program: {}", request.program);
            debug!("args: {:?}", request.args);

            // Check if client ID was not informed.
            if request.clientid == 0 {
                return Ok(Self::bad_request());
            }

            // Check if program name was not informed.
            if request.program.is_empty() {
                return Ok(Self::bad_request());
            }

            let bytes: Vec<u8> = match Self::serve(
                &sandboxes,
                request,
                requestid,
                linuxd_sockaddr,
                nanvix_console,
                sandbox_sockaddr,
                linuxd_listener,
            )
            .await
            {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!("failed to serve request ({:?})", e);
                    return Ok(Self::internal_server_error());
                },
            };

            let json: Value = serde_json::json!({
                "response": String::from_utf8_lossy(&bytes).to_string(),
            });

            // Convert JSON to bytes.
            let bytes = match serde_json::to_vec(&json) {
                Ok(bytes) => Bytes::from(bytes),
                Err(_) => {
                    let reason: String = "failed to convert JSON to bytes".to_string();
                    error!("{}", reason);
                    return Ok(Self::internal_server_error());
                },
            };

            match Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .header("Content-Length", bytes.len())
                .body(Full::new(bytes))
            {
                Ok(response) => Ok(response),
                Err(_) => {
                    let reason: String = "failed to build response".to_string();
                    error!("{}", reason);
                    Ok(Self::internal_server_error())
                },
            }
        };
        Box::pin(future)
    }
}
