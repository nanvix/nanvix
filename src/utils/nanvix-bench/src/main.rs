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
use ::sys::ipc::Message;
use anyhow::Result;
use flexi_logger::Logger;
use hwloc::HwLoc;
use indicatif::{
    ProgressBar,
    ProgressStyle,
};
use log::{
    debug,
    error,
};
use microvm::{
    Gateway,
    Vmm,
};
use mio::net::UnixStream;
use reqwest::header::{
    CONTENT_TYPE,
    HeaderMap,
};
use std::{
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
    thread,
    time::{
        Duration,
        Instant,
    },
};
use sys::pm::ThreadIdentifier;
use syscall::{
    LinuxDaemonMessage,
    unistd::message::{
        ReadRequest,
        ReadResponse,
        WriteResponse,
    },
};
use syscomm::{
    BlockingSocketStream,
    SocketStream,
    SocketType,
};

//==================================================================================================
// Constants
//==================================================================================================

// Name of this package, used for logging and error messages.
const CARGO_PKG_NAME: &str = match option_env!("CARGO_PKG_NAME") {
    Some(cargo_pkg_name) => cargo_pkg_name,
    None => "nanvix-bench",
};

/// Sleep duration (in ms) to wait for the system to clean up after a benchmark run.
const CLEANUP_SLEEP_DURATION: u64 = 10;

//==================================================================================================

const NANVIXD_ADDRESS: &str = "127.0.0.1:9999";

impl Benchmark {
    fn prepare_new_message(&self) -> Result<(HeaderMap, nanvixd::message::New)> {
        let mut new_msg_headers = HeaderMap::new();
        new_msg_headers.insert(CONTENT_TYPE, "application/json".parse()?);
        new_msg_headers.insert(
            nanvixd::config::HTTP_HEADER_MESSAGE_TYPE,
            format!("{}", nanvixd::message::MessageType::New).parse()?,
        );

        let new_msg = nanvixd::message::New {
            tenant_id: "foo".to_string(),
            app_name: "bar".to_string(),
            program: self.flavour.get_program(),
            program_args: "".to_string(),
        };

        Ok((new_msg_headers, new_msg))
    }

    fn start_nanvixd(&self) -> Result<Child> {
        let mut nanvixd_args: Vec<String> = vec![
            format!("{}/bin/nanvixd.elf", get_proj_root()),
            ::nanvixd::args::Args::OPT_HTTP_SOCKADDR.to_string(),
            NANVIXD_ADDRESS.to_string(),
            ::nanvixd::args::Args::OPT_TMP_DIRECTORY.to_string(),
            self.nanvixd_tmp_dir.clone(),
            ::nanvixd::args::Args::OPT_TOOLCHAIN_BIN_DIRECTORY.to_string(),
            self.nanvixd_toolchain_bin_dir.clone(),
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
            .current_dir(get_proj_root())
            .spawn()?;

        Ok(nanvixd_cmd)
    }

    /// Auxiliary method to start a user VM used by low-level benchmarks that want to bypass
    /// nanvixd.
    fn start_user_vm(&self, gateway_addr: Option<String>) -> Result<Child> {
        let mut user_vm_args: Vec<String> = vec![
            format!("{}/bin/microvm.elf", get_proj_root()),
            ::microvm::args::Args::OPT_KERNEL.to_string(),
            format!("{}/bin/kernel.elf", get_proj_root()),
            ::microvm::args::Args::OPT_INITRD.to_string(),
            self.flavour.get_program(),
        ];
        if let Some(gateway_addr) = gateway_addr {
            user_vm_args.push(::microvm::args::Args::OPT_SYSTEM_VM_SOCKADDR.to_string());
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
    pub fn setup(&mut self) {
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
    pub async fn start(
        &mut self,
        payload: nanvixd::message::New,
        headers: HeaderMap,
    ) -> Result<(String, BlockingSocketStream)> {
        let response: nanvixd::message::NewResponse = self
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
        let gateway_stream: SocketStream = loop {
            match SocketStream::connect(SocketType::Unix, response.gateway_sockaddr.clone()) {
                Ok(stream) => break stream,
                Err(_) => continue,
            };
        };

        let blocking_gateway_stream: BlockingSocketStream = gateway_stream.set_blocking()?;

        Ok((response.user_vm_id, blocking_gateway_stream))
    }

    /// Kill the Nano VM via POST request to nanvixd.
    pub async fn kill(&mut self, user_vm_id: String) -> Result<()> {
        let mut kill_msg_headers = HeaderMap::new();
        kill_msg_headers.insert(CONTENT_TYPE, "application/json".parse()?);
        kill_msg_headers.insert(
            nanvixd::config::HTTP_HEADER_MESSAGE_TYPE,
            format!("{}", nanvixd::message::MessageType::Kill).parse()?,
        );

        let kill_msg = nanvixd::message::Kill {
            user_vm_id: user_vm_id.clone(),
        };
        let response: nanvixd::message::KillResponse = self
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
                error!("error sending SIGINT to nano VM: {}", std::io::Error::last_os_error());
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
            // Start the clock
            let start = Instant::now();

            // The user VM will run to completion, so after starting we just
            // wait for the child process to die.
            let mut user_vm = self.start_user_vm(None)?;
            user_vm.wait()?;

            latencies.push(start.elapsed().as_micros());

            pb.inc(1);

            // Need to give some time to clean-up
            thread::sleep(Duration::from_millis(CLEANUP_SLEEP_DURATION));
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
    pub async fn run_cold_start(&mut self) -> Result<()> {
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
            self.setup();

            // Start the clock.
            let start = Instant::now();
            let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers).await?;
            gateway_stream.write_all(&payload)?;
            gateway_stream.read_exact(&mut response_payload)?;
            latencies.push(start.elapsed().as_micros());

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

            // Need to give some time to clean-up
            thread::sleep(Duration::from_millis(10));
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

    /// This function runs the warm start benchmark, where we measure the time to send a request
    /// into the VM once it has started executing.
    pub async fn run_warm_start(&mut self) -> Result<()> {
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
        self.setup();

        // Start User VM.
        let (user_vm_id, mut gateway_stream) = self.start(new_msg, new_msg_headers).await?;

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            let mut response_payload = [0u8; DATA_SIZE as usize];

            let start = Instant::now();
            gateway_stream.write_all(&payload)?;
            gateway_stream.read_exact(&mut response_payload)?;
            latencies.push(start.elapsed().as_micros());

            // Sanity-check the message to make sure is the same we sent.
            if response_payload != payload {
                error!("received payload does not match sent payload!");
                error!(" - sent: {payload:?}");
                error!(" - got: {response_payload:?}");
            }

            pb.inc(1);
        }

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
    pub fn run_warm_start_vmm(&mut self) -> Result<()> {
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

        // Initialize a UNIX pair that is directly connected to the VMM.
        let (input_stream, vmm_stream) = UnixStream::pair()?;
        let mut input_stream = syscomm::SocketStream::Unix(input_stream);

        // Spawn the VMM in a separate thread.
        let program = self.flavour.get_program();
        let vmm_handle = std::thread::spawn(move || -> Result<()> {
            match Vmm::spawn(
                config::kernel::MEMORY_SIZE,
                format!("{}/bin/kernel.elf", get_proj_root()).as_str(),
                Some(program),
                None,
                Some("/dev/null".to_string()),
                Some(Gateway::new(syscomm::SocketStream::Unix(vmm_stream))),
            )? {
                e if e != 0 => {
                    error!("error running VMM, exited with status: {e}");
                    Err(anyhow::anyhow!("VMM error"))
                },
                _ => {
                    debug!("VMM: done running");
                    Ok(())
                },
            }
        });

        let mut latencies: Vec<u128> = Vec::with_capacity(self.iterations);
        for _ in 0..self.iterations {
            // Before starting the timer, we need to receive the ReadRequest from the user VM.
            let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
                [0u8; config::kernel::IPC_MESSAGE_SIZE];

            // Explicitly spin-loop when receiving to isolate the overheads of the VMM.
            let mut num_read = 0;
            loop {
                match input_stream.try_read_exact(&mut buf[num_read..]) {
                    Ok(n) => {
                        num_read += n;
                        if num_read == buf.len() {
                            break;
                        }
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "error reading from input VMM stream (error={e:?})"
                        ));
                    },
                }
            }

            let ipc_read_message: Message = match Message::try_from_bytes(buf) {
                Ok(message) => message,
                Err(_) => return Err(anyhow::anyhow!("Error parsing buffer to IPC Read message")),
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
            input_stream.write_all(&read_response.to_bytes())?;

            // Explicitly spin-loop when receiving to isolate the overheads of the VMM.
            let mut num_read = 0;
            loop {
                match input_stream.try_read_exact(&mut buf[num_read..]) {
                    Ok(n) => {
                        num_read += n;
                        if num_read == buf.len() {
                            break;
                        }
                    },
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        let reason: String =
                            format!("error reading from input VMM stream (error={e:?})");
                        return Err(anyhow::anyhow!(reason));
                    },
                }
            }

            latencies.push(start.elapsed().as_micros());

            // After receiving the WriteRequest, we need to acknowledge it by sending a WriteResponse.
            let write_response: Message = WriteResponse::build(tid, payload.len() as i32);
            input_stream.write_all(&write_response.to_bytes())?;

            std::thread::sleep(std::time::Duration::from_millis(10));

            pb.inc(1);
        }

        // Drop the connection to send an EoF to the application code, which
        // will then gracefully shut down.
        drop(input_stream);

        // Wait for the VMM to exit after we drop the connection.
        match vmm_handle.join() {
            Ok(_) => {},
            Err(_) => return Err(anyhow::anyhow!("Error running VMM")),
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(self.iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(self.iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(self.iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    #[cfg(feature = "timestamp-messages")]
    pub fn run_echo_breakdown(&mut self) -> Result<()> {
        let steps: Vec<&str> = vec![
            // In-path
            "gateway::recv()",                          // 0
            "linuxd::handle_read_request()",            // 1
            "microvm::io::try_receive_from_gateway()",  // 2
            "microvm::io::try_send_to_microvm()",       // 3
            "microvm::mod::memory_thread::try_recv()",  // 4
            "microvm::mod::vm_input::vmexit()",         // 5
            "microvm::mod::vm_input::vm_write_bytes()", // 7
            // Out-path
            "microvm::mod::vm_output::try_send()",  // 8
            "microvm::io::try_recv_from_microvm()", // 9
            "microvm::io::try_send_to_gateway()",   // 10
            "linuxd::handle_write_request()",       // 11
            "gateway::recv()",                      // 12
        ];

        let header_size = 1;
        let data_size = header_size + profiler::MAX_NUMBER_MESSAGE_TIMESTAMPS * 2;
        let mut data = vec![0u8; data_size];

        // Before running this experiment, we need to wait for the nano VM to
        // fully boot, as otherwise the boot time will tamper the hot-path
        // measurements.
        thread::sleep(Duration::from_millis(200));

        // Add initial timestamp
        profiler::timestamp_message!(&mut data, 0);

        self.send_to_gateway(&data)?;
        let mut response = self.recv_from_gateway(data.len())?;

        // Add final timestamp
        profiler::timestamp_message!(&mut response, 0);

        // Print results
        let mut first_timestamp: Option<u16> = None;
        let mut last_timestamp: Option<u16> = None;
        let num_stamps = response[0] as usize;
        for (step_idx, chunk) in (0..num_stamps).zip(response[header_size..].chunks_exact(2)) {
            let timestamp = u16::from_le_bytes([chunk[0], chunk[1]]);

            if first_timestamp.is_none() {
                first_timestamp = Some(timestamp);
            }

            print!("{step_idx:<2} | {:<40} | Timestamp {timestamp:5} us", steps[step_idx]);

            if let Some(last) = last_timestamp {
                let delta = timestamp.wrapping_sub(last); // Handles wraparound
                println!(" | Delta {delta:5} us");
            } else {
                println!(" | First Step");
            }

            last_timestamp = Some(timestamp);
        }
        if first_timestamp.is_some() && last_timestamp.is_some() {
            println!(
                "Total time elapsed: {} us",
                last_timestamp.unwrap() - first_timestamp.unwrap()
            );
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger, and make sure we print error logs.
    Logger::try_with_env_or_str("error")
        .expect("malformed RUST_LOG environment variable")
        .start()
        .expect("failed to initialize logger");

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
        nanvixd_tmp_dir: args.tmp_dir(),
        nanvixd_toolchain_bin_dir: args.toolchain_bin_dir(),
        user_vm_id: None,
    };

    let result = match benchmark.flavour {
        BenchmarkFlavour::BootTime => {
            #[cfg(feature = "timestamp-messages")]
            {
                error!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
                return Ok(());
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_boot_time().await
            }
        },
        BenchmarkFlavour::EchoBreakdown => {
            #[cfg(not(feature = "timestamp-messages"))]
            {
                error!(
                    "WARNING: this benchmark requires Nanvix (re-) compilation with \
                     TIMESTAMP_MSG=yes"
                );
                return Ok(());
            }

            #[cfg(feature = "timestamp-messages")]
            {
                benchmark.run_echo_breakdown()
            }
        },
        BenchmarkFlavour::ColdStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                error!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
                return Ok(());
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_cold_start().await
            }
        },
        BenchmarkFlavour::WarmStart => {
            #[cfg(feature = "timestamp-messages")]
            {
                error!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
                return Ok(());
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start().await
            }
        },
        BenchmarkFlavour::WarmStartVMM => {
            #[cfg(feature = "timestamp-messages")]
            {
                error!(
                    "WARNING: this benchmark must be compiled with TIMESTAMP_MSG=no (or omit it)"
                );
                return Ok(());
            }

            #[cfg(not(feature = "timestamp-messages"))]
            {
                benchmark.run_warm_start_vmm()
            }
        },
    };
    match result {
        Ok(_) => {},
        Err(e) => error!("error running benchmark {}: {e:?}", args.benchmark()),
    }

    Ok(())
}
