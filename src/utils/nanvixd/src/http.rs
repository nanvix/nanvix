// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    args::Args,
    cache::SandboxCache,
    config,
    message::{
        self,
        MessageType,
    },
    sandbox::{
        config::SandboxConfig,
        tag::SandboxTag,
        tcp_port::{
            get_tcp_port_allocator,
            TcpPort,
        },
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
use ::std::{
    future::Future,
    pin::Pin,
    sync::Arc,
};
use ::tokio::sync::Mutex;

//==================================================================================================
// Structures
//==================================================================================================

pub struct HttpClient {
    sandbox_cache: Arc<Mutex<SandboxCache>>,
    args: Arc<Args>,
}

impl HttpClient {
    pub fn new(sandbox_cache: Arc<Mutex<SandboxCache>>, args: Arc<Args>) -> Self {
        Self {
            sandbox_cache,
            args,
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

    async fn serve_new(
        sandbox_cache: Arc<Mutex<SandboxCache>>,
        args: Arc<Args>,
        message: &message::New,
    ) -> Result<message::NewResponse> {
        let tag: SandboxTag = SandboxTag::new(&message.tenant_id, &message.app_name);
        let tmp_directory: &str = args.tmp_directory();
        let in_l2: bool = args.l2();

        let control_plane_sockaddr: String =
            config::control_plane_sockaddr_builder(tmp_directory, tag.tenant_id(), in_l2)?;
        let user_vm_sockaddr: String =
            config::user_vm_sockaddr_builder(tmp_directory, tag.tenant_id(), in_l2)?;

        // Take a lock on the sandbox cache so that we can get the next available port, and then
        // get the sandbox from the cache.
        let mut locked_sandbox_cache = sandbox_cache.lock().await;

        // Work-out the gateway address. We use one per user VM instance.
        let gateway_l2_port: Option<TcpPort> = if in_l2 {
            match get_tcp_port_allocator().lock().await.allocate().await {
                Some(port) => Some(port),
                None => {
                    let reason: String = "failed to allocate TCP port for gateway".to_string();
                    error!("{reason}");
                    return Err(::anyhow::anyhow!("{reason}"));
                },
            }
        } else {
            None
        };

        let gateway_sockaddr: String = config::gateway_sockaddr_builder(
            tmp_directory,
            tag.tenant_id(),
            tag.sandbox_id(),
            &gateway_l2_port,
        )?;

        let program_args = match message.program_args.len() {
            0 => None,
            _ => Some(message.program_args.clone()),
        };

        let config: SandboxConfig = SandboxConfig::new(
            &control_plane_sockaddr,
            &gateway_sockaddr,
            &user_vm_sockaddr,
            &message.program,
            program_args.clone(),
            args.console_file().clone(),
            args.hwloc().clone(),
            args.binary_directory(),
            args.toolchain_binary_directory(),
            args.l2(),
            // Pass ownership of the L2 gateway port, if L2 deployment enabled.
            gateway_l2_port,
        );

        // This method will create a sandbox if it is not in the cache.
        let _ = locked_sandbox_cache
            .get(&tag, Some(config), args.tmp_directory().to_string())
            .await?;

        Ok(message::NewResponse {
            user_vm_id: tag.sandbox_id(),
            gateway_sockaddr: gateway_sockaddr.clone(),
        })
    }

    async fn serve_kill(
        sandbox_cache: Arc<Mutex<SandboxCache>>,
        message: &message::Kill,
    ) -> Result<message::KillResponse> {
        let mut locked_sandbox_cache = sandbox_cache.lock().await;
        let exit_code = match locked_sandbox_cache.kill(message.user_vm_id).await {
            Ok(()) => 0,
            // TODO: more advanced error codes.
            Err(_) => 1,
        };

        Ok(message::KillResponse { exit_code })
    }
}

impl Service<Request<Incoming>> for HttpClient {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        // Clone all necessary values before moving them into the future
        let sandbox_cache: Arc<Mutex<SandboxCache>> = self.sandbox_cache.clone();
        let args: Arc<Args> = self.args.clone();
        let future = async move {
            // Get the request headers before consuming the body.
            let message_type: MessageType = match request
                .headers()
                .get(config::HTTP_HEADER_MESSAGE_TYPE)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<MessageType>().ok())
            {
                Some(message_type) => message_type,
                None => {
                    error!("{} is a mandatory header", config::HTTP_HEADER_MESSAGE_TYPE);
                    return Ok(Self::bad_request());
                },
            };

            let body: Bytes = match request.collect().await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let reason: String = "failed to read body".to_string();
                    error!("{reason}");
                    return Ok(Self::internal_server_error());
                },
            };

            // Deserialize the request body and route to the corresponding function.
            let message_response: message::MessageResponse = match message_type {
                MessageType::New => {
                    debug!("deserializing NEW message with body: {body:?}");
                    let msg: message::New = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(_) => {
                            error!("failed to deserialize NEW message: {body:?}");
                            return Ok(Self::bad_request());
                        },
                    };

                    debug!("serving NEW message:");
                    debug!("- tenant id: {}", msg.tenant_id);
                    debug!("- app name: {}", msg.app_name);
                    debug!("- program file: {}", msg.program);
                    debug!("- program args: {}", msg.program_args);

                    match Self::serve_new(sandbox_cache, args, &msg).await {
                        Ok(response) => message::MessageResponse::New(response),
                        Err(_) => {
                            error!("error processing NEW request");
                            return Ok(Self::internal_server_error());
                        },
                    }
                },
                MessageType::Kill => {
                    let msg: message::Kill = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!("failed to deserialize KILL message (error={e:?}): {body:?}");
                            return Ok(Self::bad_request());
                        },
                    };

                    debug!("serving KILL message:");
                    debug!("- user vm id: {}", msg.user_vm_id);

                    match Self::serve_kill(sandbox_cache, &msg).await {
                        Ok(response) => message::MessageResponse::Kill(response),
                        Err(_) => {
                            error!("error processing KILL request");
                            return Ok(Self::internal_server_error());
                        },
                    }
                },
            };

            // Convert response JSON to string.
            let response_string = match serde_json::to_string(&message_response) {
                Ok(string) => Bytes::from(string),
                Err(_) => {
                    error!("failed to convert JSON response to string");
                    return Ok(Self::internal_server_error());
                },
            };

            match Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Full::new(response_string))
            {
                Ok(response) => Ok(response),
                Err(_) => {
                    let reason: String = "failed to build response".to_string();
                    error!("{reason}");
                    Ok(Self::internal_server_error())
                },
            }
        };
        Box::pin(future)
    }
}
