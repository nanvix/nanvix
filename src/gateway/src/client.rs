// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::gateway::GatewayClient;
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
    server::conn::http1,
    service::Service,
    Request,
    Response,
    StatusCode,
};
use ::hyper_util::rt::TokioIo;
use ::serde::Deserialize;
use ::serde_json::Value;
use ::std::{
    collections::LinkedList,
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::Arc,
};
use ::sys::ipc::Message;
use ::tokio::{
    net::TcpStream,
    sync::{
        mpsc::{
            UnboundedReceiver,
            UnboundedSender,
        },
        RwLock,
        RwLockWriteGuard,
    },
};

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Deserialize)]
struct MessageJson {
    source: u32,
    destination: u32,
    payload: Option<Vec<u8>>,
}

///
/// # HTTP Gateway Client
///
/// A client that communicates with the gateway using HTTP.
///
pub struct HttpGatewayClient {
    /// Address of the client.
    addr: SocketAddr,
    /// Transmit endpoint for messages form the client to the gateway.
    tx: UnboundedSender<(SocketAddr, Message)>,
    /// Receive endpoint for messages from the gateway to the client.
    rx: Arc<RwLock<UnboundedReceiver<Result<Message, anyhow::Error>>>>,
}

impl HttpGatewayClient {
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

    ///
    /// # Description
    ///
    /// Converts a byte buffer to a message.
    ///
    /// # Parameters
    ///
    /// - `body`: Byte buffer.
    ///
    /// # Returns
    ///
    /// On success, a list of messages that was encoded in the byte buffer is returned. Otherwise,
    /// an error is returned instead.
    ///
    fn bytes_to_message(body: Bytes) -> Result<LinkedList<Message>> {
        // Deserialize the JSON directly into the struct
        let message_json: MessageJson = serde_json::from_slice(body.as_ref())
            .map_err(|_| anyhow::anyhow!("failed to parse request"))?;

        let payload: Vec<u8> = message_json.payload.unwrap_or_default();

        let mut messages: LinkedList<Message> = LinkedList::new();

        // Traverse the payload and create a message for each chunk.
        for chunk in payload.chunks(Message::PAYLOAD_SIZE) {
            let mut message: sys::ipc::Message = sys::ipc::Message::new(
                sys::pm::ProcessIdentifier::from(message_json.source),
                sys::pm::ProcessIdentifier::from(message_json.destination),
                sys::ipc::MessageType::Ikc,
                None,
                [0; Message::PAYLOAD_SIZE],
            );

            let len = chunk.len().min(Message::PAYLOAD_SIZE);
            message.payload[..len].copy_from_slice(&chunk[..len]);

            messages.push_back(message);
        }

        Ok(messages)
    }

    ///
    /// # Description
    ///
    /// Converts a message to a byte buffer.
    ///
    /// # Parameters
    ///
    /// - `message`: Message to convert.
    ///
    /// # Returns
    ///
    /// On success, a byte buffer that encodes the message is returned. Otherwise, an error is
    /// returned instead.
    ///
    fn message_to_bytes(message: Message) -> Result<Bytes> {
        // Convert message to JSON.
        let json: Value = serde_json::json!({
            "source": u32::from(message.source),
            "destination": u32::from(message.destination),
            "message_type": format!("{:?}", message.message_type),
            "payload": message.payload.to_vec(),
        });

        // Convert JSON to bytes.
        match serde_json::to_vec(&json) {
            Ok(bytes) => Ok(Bytes::from(bytes)),
            Err(_) => anyhow::bail!("failed to parse request"),
        }
    }
}

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl GatewayClient for HttpGatewayClient {
    fn new(
        addr: SocketAddr,
        tx: UnboundedSender<(SocketAddr, Message)>,
        rx: UnboundedReceiver<Result<Message, anyhow::Error>>,
    ) -> Self {
        HttpGatewayClient {
            addr,
            tx,
            rx: Arc::new(RwLock::new(rx)),
        }
    }

    fn run(
        client: Self,
        stream: TcpStream,
    ) -> Pin<Box<(dyn Future<Output = Result<(), anyhow::Error>> + std::marker::Send)>> {
        let io: TokioIo<TcpStream> = TokioIo::new(stream);
        Box::pin(async move {
            if let Err(e) = http1::Builder::new().serve_connection(io, client).await {
                let reason: String = format!("failed to serve connection (error={:?})", e);
                error!("run(): {}", reason);
                anyhow::bail!(reason);
            }

            Ok(())
        })
    }
}

impl Service<Request<Incoming>> for HttpGatewayClient {
    type Response = Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let lan_tx: UnboundedSender<(SocketAddr, Message)> = self.tx.clone();
        let client_rx: Arc<RwLock<UnboundedReceiver<Result<Message, anyhow::Error>>>> =
            self.rx.clone();
        let addr: SocketAddr = self.addr;
        let future = async move {
            let body: Bytes = request.collect().await?.to_bytes();

            let mut messages: LinkedList<Message> = match Self::bytes_to_message(body) {
                Ok(message) => message,
                Err(_) => {
                    let reason: String = "failed to parse request".to_string();
                    error!("{}", reason);
                    return Ok(Self::bad_request());
                },
            };

            trace!("sending {} messages", messages.len());
            while let Some(message) = messages.pop_front() {
                if let Err(e) = lan_tx.send((addr, message)) {
                    let reason: String = format!("failed to send message (error={:?})", e);
                    error!("{}", reason);
                    return Ok(Self::internal_server_error());
                }
            }

            {
                let mut client_rx: RwLockWriteGuard<
                    '_,
                    UnboundedReceiver<Result<Message, anyhow::Error>>,
                > = client_rx.write().await;
                trace!("waiting for response from the gateway");
                match client_rx.recv().await {
                    Some(Ok(message)) => {
                        let bytes: Bytes = match Self::message_to_bytes(message) {
                            Ok(bytes) => bytes,
                            Err(_) => {
                                let reason: String = "failed to parse response".to_string();
                                error!("{}", reason);
                                return Ok(Self::internal_server_error());
                            },
                        };

                        trace!("response received");
                        match Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "application/json")
                            .header("Content-Length", bytes.len())
                            .body(Full::new(bytes))
                            .map_err(|_| Self::bad_request())
                            .map_err(|_| Self::bad_request())
                        {
                            Ok(response) => Ok(response),
                            Err(_) => {
                                let reason: String = "failed to build response".to_string();
                                error!("{}", reason);
                                Ok(Self::bad_request())
                            },
                        }
                    },
                    Some(Err(e)) => {
                        let reason: String = format!("failed to receive message (error={:?})", e);
                        error!("{}", reason);
                        Ok(Self::bad_request())
                    },
                    None => {
                        let reason: String = "channel has been disconnected".to_string();
                        error!("{}", reason);
                        Ok(Self::internal_server_error())
                    },
                }
            }
        };
        Box::pin(future)
    }
}
