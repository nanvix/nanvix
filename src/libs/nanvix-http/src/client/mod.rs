// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! HTTP client handler for Nanvix Daemon.
//!
//! This module implements the HTTP service handler that processes incoming client requests.
//! It deserializes messages, routes them to appropriate handlers (NEW, KILL), and constructs
//! JSON responses. The implementation uses Hyper's Service trait for async request handling.

//==================================================================================================
// Modules
//==================================================================================================

mod standalone;

//==================================================================================================
// Imports
//==================================================================================================

use crate::message::{
    ErrorCode,
    ErrorResponse,
};
use ::http_body_util::Full;
use ::hyper::{
    body::Bytes,
    Request,
    Response,
    StatusCode,
};
use ::log::error;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub(crate) use self::standalone::HttpClient;
pub use self::standalone::StandaloneState;

//==================================================================================================
// Implementations
//==================================================================================================

impl HttpClient {
    ///
    /// # Description
    ///
    /// Returns whether an incoming request is a readiness probe.
    ///
    /// # Parameters
    ///
    /// - `request`: Incoming HTTP request.
    ///
    /// # Returns
    ///
    /// `true` for `GET /ready`, `false` otherwise.
    ///
    fn is_ready_request<B>(request: &Request<B>) -> bool {
        request.method() == ::hyper::Method::GET && request.uri().path() == "/ready"
    }

    ///
    /// # Description
    ///
    /// Builds a successful readiness response.
    ///
    /// # Returns
    ///
    /// An empty `204 No Content` response.
    ///
    fn ready_response() -> Response<Full<Bytes>> {
        let mut response: Response<Full<Bytes>> = Response::new(Full::new(Bytes::new()));
        *response.status_mut() = StatusCode::NO_CONTENT;
        response
    }

    ///
    /// # Description
    ///
    /// Helper function that creates a JSON response with the provided payload.
    ///
    /// # Parameters
    ///
    /// - `status`: HTTP status code for the response.
    /// - `payload`: Serializable payload to include in the response body.
    ///
    /// # Returns
    ///
    /// This function returns an HTTP response with a JSON body containing the serialized payload.
    ///
    fn json_response<R: serde::Serialize>(
        status: StatusCode,
        payload: &R,
    ) -> Response<Full<Bytes>> {
        match serde_json::to_vec(payload) {
            Ok(body) => match Response::builder()
                .status(status)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(body)))
            {
                Ok(response) => response,
                Err(error) => {
                    error!("failed to build response (error={error})");
                    Self::empty_response(StatusCode::INTERNAL_SERVER_ERROR)
                },
            },
            Err(error) => {
                error!("failed to serialize response (error={error})");
                Self::empty_response(StatusCode::INTERNAL_SERVER_ERROR)
            },
        }
    }

    ///
    /// # Description
    ///
    /// Helper function that returns an empty response with a given status code.
    ///
    /// # Parameters
    ///
    /// - `status`: HTTP status code for the response.
    ///
    /// # Returns
    ///
    /// This function returns an HTTP response with no body and the specified status code.
    ///
    fn empty_response(status: StatusCode) -> Response<Full<Bytes>> {
        let mut response: Response<Full<Bytes>> = Response::new(Full::new(Bytes::new()));
        *response.status_mut() = status;
        response
    }

    ///
    /// # Description
    ///
    /// Helper that wraps an `ErrorResponse` payload.
    ///
    /// # Parameters
    ///
    /// - `status`: HTTP status code for the response.
    /// - `code`: Short machine-readable error code.
    /// - `message`: Human-readable error message.
    ///
    /// # Returns
    ///
    /// This function returns an HTTP response with a JSON body containing the error details.
    ///
    fn error_response(
        status: StatusCode,
        code: ErrorCode,
        message: String,
    ) -> Response<Full<Bytes>> {
        let payload: ErrorResponse = ErrorResponse { code, message };
        Self::json_response(status, &payload)
    }
}
