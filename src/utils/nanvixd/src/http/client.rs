// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP client handler for Nanvix Daemon.
//!
//! This module implements the HTTP service handler that processes incoming client requests.
//! It deserializes messages, routes them to appropriate handlers (NEW, KILL), and constructs
//! JSON responses. The implementation uses Hyper's Service trait for async request handling.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    cache::SandboxCache,
    config,
    message::{
        self,
        MessageType,
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
use ::syslog::{
    debug,
    error,
    trace,
};
use ::tokio::sync::Mutex;
use ::user_vm_api::UserVmIdentifier;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// HTTP client handler for the Nanvix Daemon.
///
/// This structure implements the Hyper Service trait to process incoming HTTP requests.
/// It deserializes request bodies, routes them to appropriate handlers based on message
/// type headers, and constructs JSON responses.
///
pub(crate) struct HttpClient {
    /// Shared handle to the sandbox cache for managing sandboxes.
    sandbox_cache: Arc<Mutex<SandboxCache>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl HttpClient {
    ///
    /// # Description
    ///
    /// Creates a new HTTP client handler with access to the sandbox cache.
    ///
    /// # Parameters
    ///
    /// - `sandbox_cache`: Shared handle to the sandbox cache.
    ///
    /// # Returns
    ///
    /// A new HTTP client handler ready to process requests.
    ///
    pub(crate) fn new(sandbox_cache: Arc<Mutex<SandboxCache>>) -> Self {
        Self { sandbox_cache }
    }

    ///
    /// # Description
    ///
    /// Helper function that creates an HTTP "Bad Request" (400) response.
    ///
    /// # Returns
    ///
    /// An HTTP response with status code 400.
    ///
    fn bad_request() -> Response<Full<Bytes>> {
        let mut bad_request: Response<Full<Bytes>> = Response::new(Full::new(Bytes::new()));
        *bad_request.status_mut() = hyper::StatusCode::BAD_REQUEST;
        bad_request
    }

    ///
    /// # Description
    ///
    /// Helper function that creates an HTTP "Internal Server Error" (500) response.
    ///
    /// # Returns
    ///
    /// An HTTP response with status code 500.
    ///
    fn internal_server_error() -> Response<Full<Bytes>> {
        let mut internal_server_error: Response<Full<Bytes>> =
            Response::new(Full::new(Bytes::new()));
        *internal_server_error.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;
        internal_server_error
    }

    ///
    /// # Description
    ///
    /// Handles a NEW request to create a new sandbox.
    ///
    /// This function retrieves or creates a sandbox matching the request parameters and returns
    /// the User VM identifier and gateway socket address for client communication.
    ///
    /// # Parameters
    ///
    /// - `sandbox_cache`: Shared handle to the sandbox cache.
    /// - `message`: NEW message containing tenant, program, and argument information.
    ///
    /// # Returns
    ///
    /// On success, returns a NewResponse containing the User VM ID and gateway socket address.
    /// On failure, returns an error describing what went wrong.
    ///
    async fn serve_new(
        sandbox_cache: Arc<Mutex<SandboxCache>>,
        message: &message::New,
    ) -> Result<message::NewResponse> {
        trace!("serve_new(): {message:?}");

        // Get (or create) sandbox.
        let (user_vm_id, gateway_sockaddr): (UserVmIdentifier, String) = sandbox_cache
            .lock()
            .await
            .get(
                &message.tenant_id,
                &message.program,
                &message.app_name,
                if message.program_args.is_empty() {
                    None
                } else {
                    Some(message.program_args.clone())
                },
            )
            .await?;

        Ok(message::NewResponse {
            user_vm_id,
            gateway_sockaddr,
        })
    }

    ///
    /// # Description
    ///
    /// Handles a KILL request to terminate an existing sandbox.
    ///
    /// This function removes the specified sandbox from the cache and terminates its associated
    /// User VM instance.
    ///
    /// # Parameters
    ///
    /// - `sandbox_cache`: Shared handle to the sandbox cache.
    /// - `message`: KILL message containing the User VM identifier to terminate.
    ///
    /// # Returns
    ///
    /// On success, returns a KillResponse with exit code 0. On failure, returns a response
    /// with a non-zero exit code.
    ///
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

                    match Self::serve_new(sandbox_cache, &msg).await {
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
