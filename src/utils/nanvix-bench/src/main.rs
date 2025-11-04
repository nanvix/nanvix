// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
// These two allows are required because we conditionally compile the benchmarks based on whether
// the right compilation flags are used.
#![allow(dead_code)]
#![allow(unreachable_code)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod benchmark;
mod env;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    args::Args,
    benchmark::{
        Benchmark,
        BenchmarkFlavour,
    },
    env::get_proj_root,
};
use ::anyhow::Result;
use ::indicatif::{
    ProgressBar,
    ProgressStyle,
};
use ::nanvix::{
    config::kernel::MEMORY_SIZE,
    http::{
        message,
        message::{
            HTTP_HEADER_MESSAGE_TYPE,
            Kill,
            KillResponse,
            MessageType,
            New,
        },
    },
    hwloc,
    hwloc::HwLoc,
    log,
    log::{
        debug,
        error,
        warn,
    },
    sandbox::UserVmIdentifier,
    sys::{
        ipc::Message,
        pm::ThreadIdentifier,
    },
    syscall::{
        LinuxDaemonMessage,
        unistd::message::{
            ReadRequest,
            ReadResponse,
            WriteResponse,
        },
    },
    syscomm::{
        ReadExact,
        SocketStream,
        SocketType,
        UnboundSocket,
        WriteAll,
    },
    uservm,
    uservm::{
        UserVm,
        UserVmArgs,
        counters::MessageCounters,
        orchestrator::{
            IoControlCommand,
            IoControlResponse,
        },
    },
};
// FIXME(#1128): We need to re-export this import for the profiler macros.
#[cfg(feature = "timestamp-messages")]
use ::nanvix::log as syslog;
use ::reqwest::header::{
    CONTENT_TYPE,
    HeaderMap,
};
use ::std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    mem,
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
use ::tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default log-level (overridden by RUST_LOG environment variable if set).
const DEFAULT_LOG_LEVEL: &str = "error";

// Name of this package, used for logging and error messages.
const CARGO_PKG_NAME: &str = match option_env!("CARGO_PKG_NAME") {
    Some(cargo_pkg_name) => cargo_pkg_name,
    None => "nanvix-bench",
};

///
/// # Description
///
/// Sleep duration (in ms) to wait for the system to clean up after a benchmark run.
///
const CLEANUP_SLEEP_DURATION: u64 = 10;

///
/// # Description
///
/// Sleep duration (in ms) to wait for the system to clean up after a benchmark run when deploying
/// linuxd in an L2 VM. We need a longer clean-up for cloud-hypervisor to shutdown.
///
const CLEANUP_L2_SLEEP_DURATION: u64 = 100;

///
/// # Description
///
/// Maximum number messages that can be queued in a channel.
///
pub const CHANNEL_CAPACITY: usize = 1024;

//==================================================================================================

const NANVIXD_ADDRESS: &str = "127.0.0.1:9999";

impl Benchmark {
    fn prepare_new_message(&self) -> Result<(HeaderMap, message::New)> {
        let mut new_msg_headers = HeaderMap::new();
        new_msg_headers.insert(CONTENT_TYPE, "application/json".parse()?);
        new_msg_headers
            .insert(HTTP_HEADER_MESSAGE_TYPE, format!("{}", message::MessageType::New).parse()?);

        let new_msg = message::New {
            tenant_id: "foo".to_string(),
            app_name: "bar".to_string(),
            program: self.flavour.get_program(),
            program_args: "".to_string(),
        };

        Ok((new_msg_headers, new_msg))
    }

    /// Start nanvixd and, optionally, configure it to deploy linuxd inside an L2 VM.
    fn start_nanvixd(&self, l2: bool) -> Result<Child> {
        let mut nanvixd_args: Vec<String> = vec![
            format!("{}/bin/nanvixd.elf", get_proj_root()),
            ::nanvixd::args::Args::OPT_HTTP_SOCKADDR.to_string(),
            NANVIXD_ADDRESS.to_string(),
            ::nanvixd::args::Args::OPT_TOOLCHAIN_BIN_DIRECTORY.to_string(),
            self.nanvixd_toolchain_bin_dir.clone(),
        ];
        if let Some(hwloc_file) = &self.hwloc_file {
            nanvixd_args.push(::nanvixd::args::Args::OPT_HWLOC.to_string());
            nanvixd_args.push(hwloc_file.clone());
        }
        if l2 {
            nanvixd_args.push(::nanvixd::args::Args::OPT_L2.to_string());
        }

        debug!("Starting nanvixd with command: {}", nanvixd_args.join(" "));
        let nanvixd_cmd = Command::new(&nanvixd_args[0])
            .args(&nanvixd_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .current_dir(get_proj_root())
            .spawn()?;

        Ok(nanvixd_cmd)
    }

    /// Auxiliary method to start a user VM used by low-level benchmarks that want to bypass
    /// nanvixd.
    fn start_user_vm(&self, gateway_addr: Option<String>) -> Result<Child> {
        let mut user_vm_args: Vec<String> = vec![
            format!("{}/bin/uservm.elf", get_proj_root()),
            uservm::args::Args::OPT_USER_VM_ID.to_string(),
            "1".to_string(),
            uservm::args::Args::OPT_KERNEL.to_string(),
            format!("{}/bin/kernel.elf", get_proj_root()),
            uservm::args::Args::OPT_INITRD.to_string(),
            self.flavour.get_program(),
        ];
        if let Some(gateway_addr) = gateway_addr {
            user_vm_args.push(uservm::args::Args::OPT_SYSTEM_VM_SOCKADDR.to_string());
            user_vm_args.push(gateway_addr);
        }
        if let Some(hwloc) = self.hwloc.clone() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_nanovm_core_str(),
            ];
            user_vm_args.splice(0..0, taskset);
        }

        debug!("Starting user VM with command: {}", user_vm_args.join(" "));
        let user_vm_cmd = Command::new(&user_vm_args[0])
            .args(&user_vm_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .current_dir(get_proj_root())
            .spawn()?;

        Ok(user_vm_cmd)
    }

    /// Configures the set-up by starting linuxd and the gateway server.
    pub fn setup(&mut self, l2: bool) {
        match self.start_nanvixd(l2) {
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
    pub async fn start(
        &mut self,
        payload: message::New,
        headers: HeaderMap,
        l2: bool,
    ) -> Result<(UserVmIdentifier, SocketStream)> {
        let response: message::NewResponse = self
            .nanvixd_client
            .post(format!("http://{}", NANVIXD_ADDRESS))
            .headers(headers)
            .json(&payload)
            .send()
            .await?
            .json()
            .await?;

        debug!("got: user vm ID={}, gw socket={}", response.user_vm_id, response.gateway_sockaddr);

        // TODO: we need to connect the SocketStream after creating the user VM (and thus adding to
        // the cold-start time) because currently nanvixd determines the gateway address at
        // deployment time.
        let gateway_socktype: SocketType = if l2 {
            SocketType::Tcp
        } else {
            SocketType::Unix
        };
        let gateway_stream: SocketStream = loop {
            let unbound_socket: UnboundSocket = UnboundSocket::new(gateway_socktype);
            match unbound_socket.connect(&response.gateway_sockaddr).await {
                Ok(stream) => break stream,
                Err(_) => continue,
            };
        };

        Ok((response.user_vm_id, gateway_stream))
    }

    /// Kill the Nano VM via POST request to nanvixd.
    pub async fn kill(&mut self, user_vm_id: UserVmIdentifier) -> Result<()> {
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
    pub fn cleanup(&mut self) {
        if self.nanvixd.is_some() {
            debug!("Sending SIGINT to nanvixd");
            let ret_code = unsafe {
                libc::kill(self.nanvixd.as_mut().unwrap().id() as libc::pid_t, libc::SIGINT)
            };

            if ret_code < 0 {
                error!("error sending SIGINT to nanvixd: {}", std::io::Error::last_os_error());
            }

            if let Some(nanvixd) = self.nanvixd.as_mut() {
                match nanvixd.wait() {
                    Ok(exit_status) => {
                        if !exit_status.success() {
                            error!(
                                "nanvixd returned with non-zero exit status: {:?}",
                                exit_status.code()
                            );
                        }
                    },
                    Err(e) => error!("error waiting for nanvixd: {e:?}"),
                }
            }
        }
    }

    /// This function runs the boot-time experiment, where we measure the time to start a user VM
    /// with a noop application and exit. To properly isolate just the time to start a user VM, we
    /// do not make use of nanvixd here. Instead, we start the user VM manually.
    pub async fn run_boot_time(&mut self) -> Result<()> {
        // Display a progress bar
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
                mpsc::channel::<Message>(CHANNEL_CAPACITY);
            let stdout_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while vcpu_thread_stdout_rx.recv().await.is_some() {} },
                );

            let (io_control_command_tx, io_control_rx) =
                mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
            let (io_control_tx, mut io_control_response_rx) =
                mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);
            let io_response_drain: JoinHandle<()> =
                ::tokio::spawn(
                    async move { while io_control_response_rx.recv().await.is_some() {} },
                );

            let (io_thread_data_tx, memory_thread_data_rx) =
                mpsc::channel::<Message>(CHANNEL_CAPACITY);

            let kernel_filename: String = format!("{}/bin/kernel.elf", get_proj_root());
            let initrd_filename: String = self.flavour.get_program();

            // Create shared counters for tracking message flow across threads.
            let counters: MessageCounters = MessageCounters::new();

            let start = Instant::now();
            let user_vm_handle = UserVm::spawn(UserVmArgs {
                memory_size: MEMORY_SIZE,
                kernel_filename,
                initrd_filename: Some(initrd_filename),
                initrd_args: None,
                stderr: Some("/dev/null".to_string()),
                vcpu_thread_stdout_tx,
                memory_thread_data_rx,
                io_control_rx,
                io_control_tx,
                counters,
            });

            let join_result = user_vm_handle.await;

            drop(io_thread_data_tx);
            drop(io_control_command_tx);

            if let Err(error) = stdout_drain.await {
                error!("error draining user VM stdout channel: {error:?}");
            }
            if let Err(error) = io_response_drain.await {
                error!("error draining user VM control channel: {error:?}");
            }

            match join_result {
                Ok(Ok(exit_status)) => {
                    if exit_status != 0 {
                        let reason: String =
                            format!("error running user VM, exit-status={exit_status}");
                        error!("{reason}");
                        return Err(anyhow::anyhow!(reason));
                    }
                    debug!("User VM: done running");
                },
                Ok(Err(error)) => {
                    error!("error running user VM: {error:?}");
                    return Err(error);
                },
                Err(error) => {
                    let reason: String = format!("error joining user VM task: {error:?}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                },
            }

            latencies.push(start.elapsed().as_micros());

            pb.inc(1);

            // Need to give some time to clean-up
            sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations * 50) / 100]);
        println!("p95: {} us", latencies[(self.iterations * 95) / 100]);
        println!("p99: {} us", latencies[(self.iterations * 99) / 100]);

        Ok(())
    }

    /// This function runs the cold-start experiment, where we measure the time to start linuxd,
    /// start a VM, and send a request to the new VM.
    pub async fn run_cold_start(&mut self, l2: bool) -> Result<()> {
        // Display a progress bar
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Payload we are sending over the wire
        const DATA_SIZE: u32 = 10;
        let payload = [7u8; DATA_SIZE as usize];
        let mut response_payload = [0u8; DATA_SIZE as usize];

        let (new_msg_headers, new_msg) = self.prepare_new_message()?;

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            // Clone all messages we need before starting the clock.
            let new_msg_headers = new_msg_headers.clone();
            let new_msg = new_msg.clone();

            // We need to start nanvixd in each iteration of the loop, as otherwise re-using the
            // same nanvixd instance but with different linuxd instances can exhaust resource
            // limits (like open file descriptors).
            self.setup(l2);

            // Start the clock.
            let user_vm_id = {
                let start = Instant::now();
                let (user_vm_id, mut gateway_stream) =
                    self.start(new_msg, new_msg_headers, l2).await?;
                gateway_stream.write_all(&payload).await?;
                gateway_stream.read_exact(&mut response_payload).await?;
                latencies.push(start.elapsed().as_micros());
                user_vm_id
            };

            // Sanity-check the message to make sure is the same we sent.
            if response_payload != payload {
                error!("received payload does not match sent payload!");
                error!(" - sent: {payload:?}");
                error!(" - got: {response_payload:?}");
            }

            // Kill the user VM.
            self.kill(user_vm_id).await?;

            // Stop nanvixd.
            self.cleanup();

            pb.inc(1);

            // Need to give some time to clean-up (a bit longer for L2 benchmarks).
            if l2 {
                sleep(Duration::from_millis(CLEANUP_L2_SLEEP_DURATION)).await;
            } else {
                sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
            }
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        print!("Cleaning up...");
        println!("done!");

        Ok(())
    }

    ///
    /// # Description
    ///
    /// This function runs the round-trip latency benchmark, where we measure the latency of
    /// sending one message and getting it back, as we increase the message size.
    ///
    /// Given that we have many message sizes (hence many rows in the result table) we default to
    /// reporting only one percentile.
    ///
    /// # Arguments
    ///
    /// - `l2`: whether to deploy linuxd in an L2 VM or not.
    /// - `percentile`: what percentile to report.
    ///
    pub async fn run_round_trip_latency(&mut self, l2: bool) -> Result<()> {
        let message_sizes: Vec<(&str, u64)> = vec![
            ("32 B", 32),
            ("64 B", 64),
            ("128 B", 128),
            ("256 B", 256),
            ("512 B", 512),
            ("1 KiB", 1024),
            ("4 KiB", 4 * 1024),
        ];

        // Display a progress bar
        let total_num_iters: usize = self.iterations * message_sizes.len();
        let pb: ProgressBar = ProgressBar::new(total_num_iters as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        let (new_msg_headers, new_msg): (HeaderMap, New) = self.prepare_new_message()?;

        // Start nanvixd.
        self.setup(l2);

        let mut latencies: HashMap<&str, Vec<u128>> = HashMap::new();
        let user_vm_id = {
            // Start User VM.
            let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers, l2).await?;

            // Iterate over all possible message sizes.
            for (label, message_size) in &message_sizes {
                let payload: Vec<u8> = vec![7u8; *message_size as usize];

                // For each message size send many messages to get statistically relevant results.
                for _ in 0..self.iterations {
                    let mut response_payload: Vec<u8> = vec![0u8; *message_size as usize];

                    let start: Instant = Instant::now();
                    gateway_stream.write_all(&payload).await?;
                    gateway_stream.read_exact(&mut response_payload).await?;
                    latencies
                        .entry(label)
                        .or_default()
                        .push(start.elapsed().as_micros());

                    // Sanity-check the message to make sure is the same we sent.
                    if response_payload != payload {
                        error!("received payload does not match sent payload!");
                        error!(" - sent: {payload:?}");
                        error!(" - got: {response_payload:?}");
                    }

                    pb.inc(1);
                    sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
                }
            }
            user_vm_id
        };

        // Kill the user VM.
        self.kill(user_vm_id).await?;

        // Stop nanvixd.
        self.cleanup();

        pb.finish();
        println!("Size:\tp50\tp95\tp99 [us]");
        // Iterate over the message size list to print the labels in order.
        for (label, _) in message_sizes.iter() {
            if let Some(latencies) = latencies.get_mut(label) {
                latencies.sort();
                let p50: u128 = latencies[(self.iterations as f32 * 0.5) as usize];
                let p95: u128 = latencies[(self.iterations as f32 * 0.95) as usize];
                let p99: u128 = latencies[(self.iterations as f32 * 0.99) as usize];
                println!("{label}:\t{p50}\t{p95}\t{p99}");
            } else {
                warn!("missing latencies for message size: {label}");
            }
        }

        Ok(())
    }

    /// This function runs the warm start benchmark, where we measure the time to send a request
    /// into the VM once it has started executing.
    pub async fn run_warm_start(&mut self, l2: bool) -> Result<()> {
        // Display a progress bar
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Payload we are sending over the wire
        const DATA_SIZE: u32 = 10;
        let payload = [7u8; DATA_SIZE as usize];

        let (new_msg_headers, new_msg) = self.prepare_new_message()?;

        // Start nanvixd.
        self.setup(l2);

        // Start User VM.
        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        let user_vm_id = {
            let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers, l2).await?;

            for _ in 0..self.iterations {
                let mut response_payload = [0u8; DATA_SIZE as usize];

                let start = Instant::now();
                gateway_stream.write_all(&payload).await?;
                gateway_stream.read_exact(&mut response_payload).await?;
                latencies.push(start.elapsed().as_micros());

                // Sanity-check the message to make sure is the same we sent.
                if response_payload != payload {
                    error!("received payload does not match sent payload!");
                    error!(" - sent: {payload:?}");
                    error!(" - got: {response_payload:?}");
                }

                pb.inc(1);
                sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;
            }
            user_vm_id
        };

        // Kill the user VM.
        self.kill(user_vm_id).await?;

        // Stop nanvixd.
        self.cleanup();

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    /// In this micro-benchmark we measure the time for a message to travel
    /// all the way from the VMM to the guest application and back. To achieve
    /// this, we connect the user VM to a gateway that emulates linuxd.
    pub async fn run_warm_start_vmm(&mut self) -> Result<()> {
        // Display a progress bar.
        let pb = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        // Payload we are sending over the wire.
        const DATA_SIZE: u32 = 10;
        let data = [7u8; DATA_SIZE as usize];
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        payload.extend_from_slice(&DATA_SIZE.to_le_bytes());
        payload.extend_from_slice(&data);
        let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
        response_buf[..payload.len()].copy_from_slice(&payload);
        let (vcpu_thread_stdout_tx, mut vcpu_thread_stdout_rx) =
            mpsc::channel::<Message>(CHANNEL_CAPACITY);
        let (io_thread_data_tx, memory_thread_data_rx) = mpsc::channel::<Message>(CHANNEL_CAPACITY);
        let (io_control_command_tx, io_control_rx) =
            mpsc::channel::<IoControlCommand>(CHANNEL_CAPACITY);
        let (io_control_tx, mut io_control_response_rx) =
            mpsc::channel::<IoControlResponse>(CHANNEL_CAPACITY);

        let kernel_filename: String = format!("{}/bin/kernel.elf", get_proj_root());
        let program: String = self.flavour.get_program();

        // Create shared counters for tracking message flow across threads.
        let counters: MessageCounters = MessageCounters::new();

        let user_vm_handle = UserVm::spawn(UserVmArgs {
            memory_size: MEMORY_SIZE,
            kernel_filename,
            initrd_filename: Some(program),
            initrd_args: None,
            stderr: Some("/dev/null".to_string()),
            vcpu_thread_stdout_tx,
            memory_thread_data_rx,
            io_control_rx,
            io_control_tx,
            counters,
        });

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let ipc_read_message: Message = match vcpu_thread_stdout_rx.recv().await {
                Some(message) => message,
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          ReadRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };
            let linuxd_message: LinuxDaemonMessage =
                match LinuxDaemonMessage::try_from_bytes(ipc_read_message.payload) {
                    Ok(message) => message,
                    Err(_) => {
                        return Err(anyhow::anyhow!(
                            "Error parsing IPC message to LinuxDaemon message"
                        ));
                    },
                };
            let tid: ThreadIdentifier = match { ipc_read_message.source }.as_id() {
                Err(tid) => tid,
                Ok(pid) => return Err(anyhow::anyhow!("unexpected message source: {pid:?}")),
            };
            let _read_request: ReadRequest = ReadRequest::from_bytes(linuxd_message.payload);
            let read_response: Message =
                ReadResponse::build(tid, payload.len() as i32, response_buf);

            // Now we are ready to push the ReadResponse, and wait for a WriteRequest as a reply.
            let start = Instant::now();
            io_thread_data_tx.send(read_response).await?;

            let _write_request: Message = match vcpu_thread_stdout_rx.recv().await {
                Some(message) => message,
                None => {
                    let reason: String = "user VM channel closed unexpectedly while waiting for \
                                          WriteRequest"
                        .to_string();
                    error!("run_warm_start_vmm(): {reason}");
                    anyhow::bail!(reason);
                },
            };

            latencies.push(start.elapsed().as_micros());

            // After receiving the WriteRequest, we need to acknowledge it by sending a WriteResponse.
            let write_response: Message = WriteResponse::build(tid, payload.len() as i32);
            io_thread_data_tx.send(write_response).await?;

            sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION)).await;

            pb.inc(1);
        }

        io_control_command_tx
            .send(IoControlCommand::Shutdown)
            .await?;
        if let Some(response) = io_control_response_rx.recv().await {
            if response != IoControlResponse::Shutdown {
                let reason: String =
                    format!("unexpected control response received during shutdown: {response:?}");
                error!("run_warm_start_vmm(): {reason}");
                anyhow::bail!(reason);
            }
        } else {
            let reason: String = "I/O control response channel closed before receiving shutdown \
                                  acknowledgment"
                .to_string();
            error!("run_warm_start_vmm(): {reason}");
            anyhow::bail!(reason);
        }

        drop(io_thread_data_tx);
        drop(io_control_command_tx);

        match user_vm_handle.await {
            Ok(Ok(exit_status)) => {
                if exit_status != 0 {
                    let reason: String =
                        format!("error running user VM, exit-status={exit_status}");
                    error!("{reason}");
                    return Err(anyhow::anyhow!(reason));
                }
                debug!("User VM: done running");
            },
            Ok(Err(error)) => {
                error!("error running user VM: {error:?}");
                return Err(error);
            },
            Err(error) => {
                let reason: String = format!("error joining user VM task: {error:?}");
                error!("{reason}");
                return Err(anyhow::anyhow!(reason));
            },
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    #[cfg(feature = "timestamp-messages")]
    pub async fn run_echo_breakdown(&mut self, l2: bool) -> Result<()> {
        // First start nanvixd and the user VM.
        let (new_msg_headers, new_msg) = self.prepare_new_message()?;
        self.setup(l2);
        let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers, l2).await?;

        // The labels in this array are also added as comments to the line of code where the
        // timestamp is added.
        let steps: Vec<&str> = vec![
            // In-path
            "nanvix-bench::write_all()",                    // 0
            "linuxd::worker_thread::handle_read_request()", // 1
            "uservm::io_thread::system_vm::read()",         // 2
            "uservm::memory_thread::data_rx::recv()",       // 3
            "uservm::lib::vm_input::vmexit()",              // 4
            "uservm::lib::vm_input::vm_write_bytes()",      // 5
            // Out-path
            "uservm::lib::vm_output::send()",                // 6
            "uservm::io_thread::system_vm::write()",         // 7
            "linuxd::worker_thread::handle_write_request()", // 8
            "nanvix-bench::read_exact()",                    // 9
        ];

        let header_size = 1;
        let data_size = header_size + profiler::MAX_NUMBER_MESSAGE_TIMESTAMPS * 2;

        // Before running this experiment, we need to wait for the user VM to
        // fully boot, as otherwise the boot time will tamper the hot-path
        // measurements.
        sleep(Duration::from_millis(200)).await;

        // For each different step we measure, we record the delta for each iteration.
        let mut latencies: Vec<Vec<u16>> = Vec::with_capacity(steps.len() + 1);
        for _ in 0..(steps.len() + 1) {
            latencies.push(vec![0u16; self.iterations]);
        }

        // Display a progress bar.
        let pb: ProgressBar = ProgressBar::new(self.iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        for iter in 0..self.iterations {
            let mut data: Vec<u8> = vec![0u8; data_size];
            let mut response: Vec<u8> = vec![0u8; data_size];

            // Add initial timestamp
            // Label: nanvix-bench::write_all()
            profiler::timestamp_message!(&mut data, 0);

            gateway_stream.write_all(&data).await?;
            gateway_stream.read_exact(&mut response).await?;

            // Add final timestamp.
            // Label: nanvix-bench::read_exact()
            profiler::timestamp_message!(&mut response, 0);

            // Process results.
            let mut first_timestamp: Option<u16> = None;
            let mut last_timestamp: Option<u16> = None;
            let num_stamps: usize = response[0] as usize;
            if num_stamps != steps.len() {
                return Err(anyhow::anyhow!(
                    "not enough timestamps (got={num_stamps}, expected={})",
                    steps.len()
                ));
            }
            for (step_idx, chunk) in (0..num_stamps).zip(response[header_size..].chunks_exact(2)) {
                let timestamp: u16 = u16::from_le_bytes([chunk[0], chunk[1]]);

                if first_timestamp.is_none() {
                    first_timestamp = Some(timestamp);
                }

                if let Some(last) = last_timestamp {
                    let delta: u16 = timestamp.wrapping_sub(last);
                    latencies[step_idx][iter] = delta;
                }

                last_timestamp = Some(timestamp);
            }

            if first_timestamp.is_some() && last_timestamp.is_some() {
                latencies[steps.len()][iter] = last_timestamp.unwrap() - first_timestamp.unwrap()
            } else {
                return Err(anyhow::anyhow!("have not collected enough timestamps!"));
            }

            pb.inc(1);
        }

        pb.finish();

        // Clean-up.
        self.kill(user_vm_id).await?;
        self.cleanup();

        // Print results
        for step_idx in 0..(steps.len() + 1) {
            if step_idx < steps.len() {
                print!("{step_idx:<2} | {:<48}", steps[step_idx]);
            } else {
                print!("{step_idx:<2} | {:<48}", "Total");
            }

            if step_idx == 0 {
                println!(" | First Step");
                continue;
            }

            latencies[step_idx].sort();
            print!(
                " | p50: {:5} | p95: {:5} | p99 {:5}",
                latencies[step_idx][(self.iterations as f32 * 0.5) as usize],
                latencies[step_idx][(self.iterations as f32 * 0.95) as usize],
                latencies[step_idx][(self.iterations as f32 * 0.99) as usize],
            );

            if step_idx < steps.len() && steps[step_idx] == "microvm::mod::vm_input::vmexit()" {
                println!(" | Time for VM to react to IO being avail.");
            } else {
                println!();
            }
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    log::init(false, DEFAULT_LOG_LEVEL, String::new());

    // Check if RELEASE=yes was set at build time.
    match option_env!("RELEASE") {
        Some("yes") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with RELEASE=yes");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    // Check if LOG_LEVEL was set at build time and ensure it is "panic".
    match option_env!("LOG_LEVEL") {
        Some("panic") => {},
        Some(_) | None => {
            let reason: String =
                format!("{CARGO_PKG_NAME} requires Nanvix to be compiled with LOG_LEVEL=panic");
            error!("{reason}");
            anyhow::bail!(reason);
        },
    }

    let args: Args = Args::parse(std::env::args().collect())?;

    // Parse hwloc from JSON file.
    let hwloc: Option<HwLoc> = if let Some(hwloc_file_path) = args.hwloc_file() {
        let hwloc_file: File = File::open(hwloc_file_path)?;
        let hwloc_reader: BufReader<File> = BufReader::new(hwloc_file);
        Some(serde_json::from_reader(hwloc_reader)?)
    } else {
        None
    };

    // Initialize HwLoc and pin main thread.
    if let Some(hwloc) = hwloc.clone() {
        hwloc::pin_main_thread(hwloc.get_client_core_str())?;
    }

    let mut benchmark = Benchmark {
        iterations: args.iterations(),
        hwloc_file: args.hwloc_file(),
        hwloc,
        flavour: args.benchmark(),
        nanvixd: None,
        nanvixd_client: reqwest::Client::new(),
        nanvixd_toolchain_bin_dir: args.toolchain_bin_dir(),
        user_vm_id: None,
    };

    let result = match benchmark.flavour {
        BenchmarkFlavour::BootTime => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_boot_time().await
            }
        },
        BenchmarkFlavour::ColdStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_cold_start(false).await
            }
        },
        BenchmarkFlavour::ColdStartL2 => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_cold_start(true).await
            }
        },
        BenchmarkFlavour::EchoBreakdown => {
            #[cfg(not(feature = "timestamp-messages"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     TIMESTAMP_MSG=yes"
                );
            }

            #[cfg(feature = "timestamp-messages")]
            {
                benchmark.run_echo_breakdown(false).await
            }
        },
        BenchmarkFlavour::EchoBreakdownL2 => {
            #[cfg(not(feature = "timestamp-messages"))]
            {
                anyhow::bail!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     TIMESTAMP_MSG=yes"
                );
            }

            #[cfg(feature = "timestamp-messages")]
            {
                benchmark.run_echo_breakdown(true).await
            }
        },
        BenchmarkFlavour::RoundTripLatency => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_round_trip_latency(false).await
            }
        },
        BenchmarkFlavour::WarmStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start(false).await
            }
        },
        BenchmarkFlavour::WarmStartL2 => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start(true).await
            }
        },
        BenchmarkFlavour::WarmStartVMM => {
            #[cfg(feature = "timestamp-messages")]
            {
                anyhow::bail!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start_vmm().await
            }
        },
    };
    match result {
        Ok(()) => {},
        Err(e) => {
            anyhow::bail!("error running benchmark {}: {e:?}", args.benchmark());

            // In case of an error, re-run the clean up to prevent having dangling processes. Note
            // that the clean up is idempotent.
            benchmark.cleanup();
        },
    }

    Ok(())
}
