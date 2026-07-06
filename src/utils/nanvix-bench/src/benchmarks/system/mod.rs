// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod cold_start;
#[cfg(feature = "timestamp-messages")]
mod echo_breakdown;
mod round_trip_latency;
mod warm_start;

//==================================================================================================
// Imports
//==================================================================================================

use super::DEFAULT_PAYLOAD_SIZE;
use crate::benchmark::Benchmark;
use ::anyhow::Result;
use ::log::{
    debug,
    error,
    warn,
};
use ::nanvix::{
    http::{
        message,
        message::{
            ErrorResponse,
            HTTP_HEADER_MESSAGE_TYPE,
            Kill,
            KillResponse,
            MessageType,
            New,
        },
    },
    sandbox::UserVmIdentifier,
    syscomm::{
        ReadExact,
        SocketStream,
        SocketType,
        UnboundSocket,
        WriteAll,
    },
};
use ::reqwest::header::{
    CONTENT_TYPE,
    HeaderMap,
};
use ::std::{
    net::TcpStream,
    process::{
        self,
        Child,
        Command,
        Stdio,
    },
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::time::{
    sleep,
    timeout,
};

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Timeout (in seconds) to wait for nanvixd to exit after SIGINT before sending SIGKILL.
///
const NANVIXD_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

///
/// # Description
///
/// Sleep duration (in milliseconds) between shutdown polling attempts while waiting for nanvixd to
/// exit.
///
const NANVIXD_SHUTDOWN_POLL_INTERVAL_MS: u64 = 100;

///
/// # Description
///
/// Timeout (in seconds) for the gateway connection retry loop. Matches the nanvixd-side gateway
/// probe timeout (`GATEWAY_CONNECT_TIMEOUT`).
///
const GATEWAY_CONNECT_TIMEOUT_SECS: u64 = 60;

///
/// # Description
///
/// Timeout (in seconds) for echo I/O operations (write + read) on the gateway stream.
///
const ECHO_IO_TIMEOUT_SECS: u64 = 60;

///
/// # Description
///
/// Sleep duration (in milliseconds) between gateway connection retry attempts.
///
const GATEWAY_CONNECT_RETRY_SLEEP_MS: u64 = 1;

///
/// # Description
///
/// Address used to communicate with nanvixd over HTTP.
///
pub(crate) const NANVIXD_ADDRESS: &str = "127.0.0.1:9999";

///
/// # Description
///
/// Default tenant identifier used when no explicit tenant is provided.
///
pub(crate) const DEFAULT_TENANT_ID: &str = "foo";

///
/// # Description
///
/// Default application name used when no explicit name is provided.
///
pub(crate) const DEFAULT_APP_NAME: &str = "bar";

//==================================================================================================
// Implementations
//==================================================================================================

impl Benchmark {
    pub(crate) fn prepare_new_message(
        &self,
        tenant_id: Option<String>,
        app_name: Option<String>,
    ) -> Result<(HeaderMap, message::New)> {
        let mut new_msg_headers = HeaderMap::new();
        new_msg_headers.insert(CONTENT_TYPE, "application/json".parse()?);
        new_msg_headers
            .insert(HTTP_HEADER_MESSAGE_TYPE, format!("{}", message::MessageType::New).parse()?);

        let new_msg = message::New {
            tenant_id: tenant_id.unwrap_or(DEFAULT_TENANT_ID.to_string()),
            app_name: app_name.unwrap_or(DEFAULT_APP_NAME.to_string()),
            program: self.flavour.get_program(&self.workspace_root),
            program_args: "".to_string(),
        };

        Ok((new_msg_headers, new_msg))
    }

    /// Start nanvixd.
    fn start_nanvixd(&self) -> Result<Child> {
        let mut nanvixd_args: Vec<String> = vec![
            format!("{}/bin/nanvixd.elf", self.workspace_root.display()),
            ::nanvixd::args::Args::OPT_HTTP_SOCKADDR.to_string(),
            NANVIXD_ADDRESS.to_string(),
            ::nanvixd::args::Args::OPT_TMP_DIRECTORY.to_string(),
            self.nanvixd_tmp_dir.clone(),
        ];
        if let Some(hwloc_file) = &self.hwloc_file {
            nanvixd_args.push(::nanvixd::args::Args::OPT_HWLOC.to_string());
            nanvixd_args.push(hwloc_file.clone());
        }

        debug!("Starting nanvixd with command: {}", nanvixd_args.join(" "));
        let nanvixd_cmd = Command::new(&nanvixd_args[0])
            .args(&nanvixd_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .current_dir(&self.workspace_root)
            .spawn()?;

        Ok(nanvixd_cmd)
    }

    /// Configures the set-up by starting linuxd and the gateway server.
    pub(crate) fn setup(&mut self) {
        match self.start_nanvixd() {
            Ok(nanvixd) => self.nanvixd = Some(nanvixd),
            Err(_) => {
                error!("error starting up nanvixd");
                self.cleanup();
                process::exit(1);
            },
        }

        while TcpStream::connect_timeout(
            &NANVIXD_ADDRESS.to_string().parse().unwrap(),
            Duration::from_millis(10),
        )
        .is_err()
        {
            continue;
        }

        debug!("nanvixd is ready to serve requests");
    }

    /// Starts the Nano VM via POST request to nanvixd. Returns the user VM ID as well as an open
    /// socket to interact with the VMs stdin/stdout.
    pub(crate) async fn start(
        &mut self,
        payload: message::New,
        headers: HeaderMap,
    ) -> Result<(UserVmIdentifier, SocketStream)> {
        let http_response: ::reqwest::Response = self
            .nanvixd_client
            .post(format!("http://{}", NANVIXD_ADDRESS))
            .headers(headers)
            .json(&payload)
            .send()
            .await?;

        // Check if the response is successful
        let status: ::reqwest::StatusCode = http_response.status();
        if !status.is_success() {
            // Try to deserialize as ErrorResponse to get detailed error info
            let error_msg: String = match http_response.json::<ErrorResponse>().await {
                Ok(err_response) => {
                    format!(
                        "nanvixd returned error (status={}, code={:?}): {}",
                        status, err_response.code, err_response.message
                    )
                },
                Err(e) => {
                    format!(
                        "nanvixd returned error (status={}): failed to parse error response: {}",
                        status, e
                    )
                },
            };
            error!("{}", error_msg);
            anyhow::bail!(error_msg);
        }

        let response: message::NewResponse = http_response.json().await?;

        debug!("got: user vm ID={}, gw socket={}", response.user_vm_id, response.gateway_sockaddr);

        // TODO: we need to connect the SocketStream after creating the user VM (and thus adding to
        // the cold-start time) because currently nanvixd determines the gateway address at
        // deployment time.
        let gateway_socktype: SocketType = SocketType::Unix;
        let gateway_stream: SocketStream = {
            let deadline: Duration = Duration::from_secs(GATEWAY_CONNECT_TIMEOUT_SECS);
            match timeout(deadline, async {
                loop {
                    let unbound_socket: UnboundSocket = UnboundSocket::new(gateway_socktype);
                    match unbound_socket.connect(&response.gateway_sockaddr).await {
                        Ok(stream) => break stream,
                        Err(_) => {
                            sleep(Duration::from_millis(GATEWAY_CONNECT_RETRY_SLEEP_MS)).await;
                            continue;
                        },
                    };
                }
            })
            .await
            {
                Ok(stream) => stream,
                Err(_) => {
                    error!(
                        "gateway connection to {} timed out after {}s",
                        response.gateway_sockaddr, GATEWAY_CONNECT_TIMEOUT_SECS
                    );
                    anyhow::bail!(
                        "gateway connection to {} timed out after {}s",
                        response.gateway_sockaddr,
                        GATEWAY_CONNECT_TIMEOUT_SECS
                    );
                },
            }
        };
        debug!("connected to gateway socket stream");

        Ok((response.user_vm_id, gateway_stream))
    }

    /// Kill the Nano VM via POST request to nanvixd.
    pub(crate) async fn kill(&mut self, user_vm_id: UserVmIdentifier) -> Result<()> {
        let mut kill_msg_headers = HeaderMap::new();
        kill_msg_headers.insert(CONTENT_TYPE, "application/json".parse()?);
        kill_msg_headers
            .insert(HTTP_HEADER_MESSAGE_TYPE, format!("{}", MessageType::Kill).parse()?);

        let kill_msg: Kill = Kill { user_vm_id };
        let response: KillResponse = self
            .nanvixd_client
            .post(format!("http://{}", NANVIXD_ADDRESS))
            .headers(kill_msg_headers)
            .json(&kill_msg)
            .send()
            .await?
            .json()
            .await?;

        if response.exit_code != 0 {
            error!("error killing user VM (id={user_vm_id}, exit-code={})", response.exit_code);
        }

        Ok(())
    }

    /// Kill the different components in order.
    pub(crate) fn cleanup(&mut self) {
        if let Some(nanvixd) = self.nanvixd.as_mut() {
            #[cfg(unix)]
            {
                debug!("Sending SIGINT to nanvixd");
                let ret_code: i32 =
                    unsafe { libc::kill(nanvixd.id() as libc::pid_t, libc::SIGINT) };

                if ret_code < 0 {
                    error!("error sending SIGINT to nanvixd: {}", std::io::Error::last_os_error());
                }

                // Wait for nanvixd to exit with a bounded timeout to prevent indefinite hangs.
                let deadline = Instant::now() + Duration::from_secs(NANVIXD_SHUTDOWN_TIMEOUT_SECS);
                loop {
                    match nanvixd.try_wait() {
                        Ok(Some(exit_status)) => {
                            if !exit_status.success() {
                                error!(
                                    "nanvixd returned with non-zero exit status: {:?}",
                                    exit_status.code()
                                );
                            }
                            break;
                        },
                        Ok(None) => {
                            if Instant::now() >= deadline {
                                warn!(
                                    "nanvixd did not exit within {}s after SIGINT, sending SIGKILL",
                                    NANVIXD_SHUTDOWN_TIMEOUT_SECS
                                );
                                let _ = unsafe {
                                    libc::kill(nanvixd.id() as libc::pid_t, libc::SIGKILL)
                                };
                                let _ = nanvixd.wait();
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(
                                NANVIXD_SHUTDOWN_POLL_INTERVAL_MS,
                            ));
                        },
                        Err(e) => {
                            error!("error waiting for nanvixd: {e:?}");
                            break;
                        },
                    }
                }
            }

            #[cfg(not(unix))]
            {
                debug!("Terminating nanvixd process");
                if let Err(e) = nanvixd.kill() {
                    error!("error terminating nanvixd: {e}");
                }

                let deadline = Instant::now() + Duration::from_secs(NANVIXD_SHUTDOWN_TIMEOUT_SECS);
                loop {
                    match nanvixd.try_wait() {
                        Ok(Some(_)) => break,
                        Ok(None) => {
                            if Instant::now() >= deadline {
                                warn!(
                                    "nanvixd did not exit within {}s after terminate",
                                    NANVIXD_SHUTDOWN_TIMEOUT_SECS
                                );
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(
                                NANVIXD_SHUTDOWN_POLL_INTERVAL_MS,
                            ));
                        },
                        Err(e) => {
                            error!("error waiting for nanvixd: {e:?}");
                            break;
                        },
                    }
                }
            }

            self.nanvixd = None;
        }
    }

    ///
    /// # Description
    ///
    /// This method spawns a user VM with the request deployment characteristics, sends an echo
    /// message and, optionally, records the time it all took, and persists the user VM's stream.
    ///
    pub(crate) async fn run_user_vm_echo_once(
        &mut self,
        new_msg_headers: HeaderMap,
        new_msg: New,
        cleanup_duration: Duration,
        latencies: Option<&mut Vec<u128>>,
        in_flight_uvms: &mut Option<Vec<(UserVmIdentifier, SocketStream)>>,
    ) -> Result<()> {
        let payload: [u8; DEFAULT_PAYLOAD_SIZE] = [7u8; DEFAULT_PAYLOAD_SIZE];
        let mut response_payload: [u8; DEFAULT_PAYLOAD_SIZE] = [0u8; DEFAULT_PAYLOAD_SIZE];

        let start: Instant = Instant::now();
        let (user_vm_id, mut gateway_stream): (UserVmIdentifier, SocketStream) =
            self.start(new_msg, new_msg_headers).await?;
        let echo_io_timeout: Duration = Duration::from_secs(ECHO_IO_TIMEOUT_SECS);
        timeout(echo_io_timeout, gateway_stream.write_all(&payload))
            .await
            .map_err(|_| {
                error!("echo write timed out after {}s", ECHO_IO_TIMEOUT_SECS);
                anyhow::anyhow!("echo write timed out after {}s", ECHO_IO_TIMEOUT_SECS)
            })?
            .map_err(|e| anyhow::anyhow!("echo write failed: {e}"))?;
        timeout(echo_io_timeout, gateway_stream.read_exact(&mut response_payload))
            .await
            .map_err(|_| {
                error!("echo read timed out after {}s", ECHO_IO_TIMEOUT_SECS);
                anyhow::anyhow!("echo read timed out after {}s", ECHO_IO_TIMEOUT_SECS)
            })?
            .map_err(|e| anyhow::anyhow!("echo read failed: {e}"))?;
        let elapsed_micros: u128 = start.elapsed().as_micros();

        // Only record latency if requested to.
        if let Some(latencies) = latencies {
            latencies.push(elapsed_micros);
        }

        // Sanity-check the message to make sure is the same we sent.
        if response_payload != payload {
            error!("received payload does not match sent payload!");
            error!(" - sent: {payload:?}");
            error!(" - got: {response_payload:?}");
        }

        // If we must persist, store the user VM id and gateway stream. Otherwise clean-up.
        if let Some(in_flight_uvms) = in_flight_uvms.as_mut() {
            in_flight_uvms.push((user_vm_id, gateway_stream));
        } else {
            // Kill the user VM.
            self.kill(user_vm_id).await?;
        }

        sleep(cleanup_duration).await;

        Ok(())
    }
}

#[cfg(not(feature = "timestamp-messages"))]
impl Benchmark {
    pub async fn run_echo_breakdown(&mut self) -> Result<()> {
        anyhow::bail!("echo-breakdown requires compilation with timestamp-messages feature")
    }
}
