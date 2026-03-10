// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Multi-process deployment mode implementation for the HTTP client.

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    self,
    ErrorCode,
    MessageType,
    HTTP_HEADER_MESSAGE_TYPE,
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
use ::log::{
    debug,
    error,
    trace,
};
use ::nanvix_sandbox_cache::SandboxCache;
use ::std::{
    future::Future,
    pin::Pin,
    sync::Arc,
};
use ::syscomm::SocketType;
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
/// # Type Parameters
///
/// - `T`: Custom state type for the syscall table. This is passed to system call handlers in
///   single-process mode. Must implement `Send + Sync + Default`. Use `()` if no custom state is required.
///
pub(crate) struct HttpClient<T> {
    /// Shared handle to the sandbox cache for managing sandboxes.
    sandbox_cache: Arc<SandboxCache<T>>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<T: Send + Sync + Default + 'static> super::HttpClient<T> {
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
    pub(crate) fn new(sandbox_cache: Arc<SandboxCache<T>>) -> Self {
        Self { sandbox_cache }
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
    /// On success, returns a `NewResponse` containing the User VM ID and gateway socket address.
    /// On failure, returns an error describing what went wrong.
    ///
    pub(super) async fn serve_new(
        sandbox_cache: Arc<SandboxCache<T>>,
        message: &message::New,
    ) -> Result<message::NewResponse> {
        trace!("serve_new(): {message:?}");

        // Get (or create) sandbox.
        let (user_vm_id, gateway_sockaddr, _gateway_socket_type): (
            UserVmIdentifier,
            String,
            SocketType,
        ) = sandbox_cache
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
    /// On success, returns an acknowledgement for the terminated sandbox. On failure, this function
    /// returns an object that describes the error.
    ///
    pub(super) async fn serve_kill(
        sandbox_cache: Arc<SandboxCache<T>>,
        message: &message::Kill,
    ) -> Result<message::KillResponse> {
        match sandbox_cache.kill(message.user_vm_id).await {
            Ok(exit_code) => Ok(message::KillResponse { exit_code }),
            Err(error) => {
                error!(
                    "failed to terminate sandbox (user_vm_id={} error={error})",
                    message.user_vm_id,
                );
                Err(error)
            },
        }
    }
}

impl<T: Send + Sync + Default + 'static> Service<Request<Incoming>> for HttpClient<T> {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        // Clone all necessary values before moving them into the future
        let sandbox_cache = self.sandbox_cache.clone();
        let future = async move {
            // Get the request headers before consuming the body.
            let message_type: MessageType = match request
                .headers()
                .get(HTTP_HEADER_MESSAGE_TYPE)
                .and_then(|val| val.to_str().ok())
                .and_then(|s| s.parse::<MessageType>().ok())
            {
                Some(message_type) => message_type,
                None => {
                    let message: String =
                        format!("{} is a mandatory header", HTTP_HEADER_MESSAGE_TYPE);
                    error!("{message}");
                    return Ok(Self::error_response(
                        StatusCode::BAD_REQUEST,
                        ErrorCode::MissingMessageType,
                        message,
                    ));
                },
            };

            let body: Bytes = match request.collect().await {
                Ok(body) => body.to_bytes(),
                Err(_) => {
                    let reason: String = "failed to read body".to_string();
                    error!("{reason}");
                    return Ok(Self::error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ErrorCode::BodyReadFailed,
                        reason,
                    ));
                },
            };

            // Deserialize the request body and route to the corresponding function.
            let message_response: message::MessageResponse = match message_type {
                MessageType::New => {
                    debug!("deserializing NEW message with body: {body:?}");
                    let msg: message::New = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(_) => {
                            let reason: String =
                                format!("failed to deserialize NEW message: {body:?}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::BAD_REQUEST,
                                ErrorCode::InvalidNewPayload,
                                reason,
                            ));
                        },
                    };

                    match Self::serve_new(sandbox_cache, &msg).await {
                        Ok(response) => message::MessageResponse::New(response),
                        Err(error) => {
                            let reason: String = format!("failed to process NEW request: {error}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                ErrorCode::NewRequestFailed,
                                reason,
                            ));
                        },
                    }
                },
                MessageType::Kill => {
                    let msg: message::Kill = match serde_json::from_slice(&body) {
                        Ok(msg) => msg,
                        Err(e) => {
                            let reason: String = format!(
                                "failed to deserialize KILL message (error={e:?}): {body:?}"
                            );
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::BAD_REQUEST,
                                ErrorCode::InvalidKillPayload,
                                reason,
                            ));
                        },
                    };

                    debug!("serving KILL message:");
                    debug!("- user vm id: {}", msg.user_vm_id);

                    match Self::serve_kill(sandbox_cache, &msg).await {
                        Ok(response) => message::MessageResponse::Kill(response),
                        Err(error) => {
                            let reason: String = format!("failed to process KILL request: {error}");
                            error!("{reason}");
                            return Ok(Self::error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                ErrorCode::KillRequestFailed,
                                reason,
                            ));
                        },
                    }
                },
            };

            Ok(Self::json_response(StatusCode::OK, &message_response))
        };
        Box::pin(future)
    }
}
