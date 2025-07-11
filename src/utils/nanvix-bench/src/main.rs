// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]
// These two allows are required because we conditionally compile the benchmarks based on wether
// the right compilation flags are used.
#![allow(dead_code)]
#![allow(unreachable_code)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;
mod benchmark;
mod hwloc;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    args::Args,
    benchmark::{
        Benchmark,
        BenchmarkFlavour,
    },
};
use ::sys::ipc::Message;
use anyhow::Result;
use flexi_logger::Logger;
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
use nix::{
    sys::signal::{
        Signal,
        kill,
    },
    unistd::Pid,
};
use std::{
    env,
    fs,
    io::{
        ErrorKind,
        Read,
        Write,
    },
    mem,
    net::TcpStream,
    os::unix::net::UnixStream,
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

//==================================================================================================

const NANVIX_LINUXD_UNIX_SOCKET: &str = "/tmp/nanvix_ubench.socket";
const GATEWAY_ADDRESS: &str = "127.0.0.1:9999";

fn get_proj_root() -> String {
    format!("{}/../../..", env!("CARGO_MANIFEST_DIR"))
}

impl Benchmark {
    fn start_gateway(&mut self) -> Result<TcpStream> {
        debug!("Connecting gateway to {}...", &self.gateway_address);
        let stream: TcpStream = loop {
            match TcpStream::connect(&self.gateway_address) {
                Ok(stream) => break stream,
                Err(_) => {
                    continue;
                },
            };
        };
        debug!("Connected!");

        Ok(stream)
    }

    /// Send message to gateway by prepending the message size as a u32 LE.
    fn send_to_gateway(&mut self, data: &[u8]) -> Result<()> {
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        let data_len: u32 = data.len().try_into().unwrap();
        payload.extend_from_slice(&data_len.to_le_bytes());
        payload.extend_from_slice(data);

        Ok(self.gateway.as_mut().unwrap().write_all(&payload)?)
    }

    /// Read message from gateway by first parsing the length as an u32 LE.
    fn recv_from_gateway(&mut self, data_size: usize) -> Result<Vec<u8>> {
        let mut response_payload: Vec<u8> = vec![0u8; mem::size_of::<u32>() + data_size];
        self.gateway
            .as_mut()
            .unwrap()
            .read_exact(&mut response_payload)?;

        Ok(response_payload[mem::size_of::<u32>()..].to_vec())
    }

    fn start_linuxd(&self) -> Result<Child> {
        let mut linuxd_args: Vec<String> = vec![
            format!("{}/bin/linuxd.elf", get_proj_root()),
            "-user-vm-bind-addr".to_string(),
            self.linuxd_address.clone(),
            "-gateway-bind-addr".to_string(),
            self.gateway_address.to_string(),
            "-gateway-bind-socket-type".to_string(),
            "tcp".to_string(),
        ];
        if let Some(hwloc) = &self.hwloc {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                hwloc.get_linuxd_core_str(),
            ];
            linuxd_args.splice(0..0, taskset);
        }

        debug!("Starting linuxd with command: {}", linuxd_args.join(" "));
        let linuxd_cmd = Command::new(&linuxd_args[0])
            .args(&linuxd_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(get_proj_root())
            .spawn()?;

        Ok(linuxd_cmd)
    }

    fn start_nanovm(&self) -> Result<Child> {
        let mut nanovm_args: Vec<String> = vec![
            format!("{}/bin/microvm.elf", get_proj_root()),
            "-kernel".to_string(),
            format!("{}/bin/kernel.elf", get_proj_root()),
            "-initrd".to_string(),
            match self.flavour {
                BenchmarkFlavour::ColdStart => {
                    format!("{}/bin/echo-rust-nostd.elf", get_proj_root())
                },
                BenchmarkFlavour::WarmStart => {
                    format!("{}/bin/echo-rust-server-nostd.elf", get_proj_root())
                },
                BenchmarkFlavour::WarmStartVMM | BenchmarkFlavour::EchoBreakdown => {
                    format!("{}/bin/echo-single-rust-nostd.elf", get_proj_root())
                },
            },
            "-gateway".to_string(),
            self.linuxd_address.clone(),
        ];
        if self.hwloc.is_some() {
            let taskset: Vec<String> = vec![
                "taskset".to_string(),
                "-ac".to_string(),
                self.hwloc.clone().unwrap().get_nanovm_core_str(),
            ];
            nanovm_args.splice(0..0, taskset);
        }

        debug!("Starting nano VM with command: {}", nanovm_args.join(" "));
        let nanovm_cmd = Command::new(&nanovm_args[0])
            .args(&nanovm_args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(get_proj_root())
            .spawn()?;

        Ok(nanovm_cmd)
    }

    /// Configures teh set-up by starting linuxd and the gateway server.
    pub fn setup(&mut self) {
        match self.start_linuxd() {
            Ok(linuxd) => self.linuxd = Some(linuxd),
            Err(_) => {
                error!("error starting up linuxd");
                self.cleanup();
                process::exit(1);
            },
        }
        match self.start_gateway() {
            Ok(gateway) => self.gateway = Some(gateway),
            Err(_) => {
                error!("error starting up the gateway");
                self.cleanup();
                process::exit(1);
            },
        }
    }

    /// Starts the Nano VM.
    pub fn start(&mut self) {
        match self.start_nanovm() {
            Ok(nanovm) => self.nanovm = Some(nanovm),
            Err(_) => {
                error!("error starting up nano vm");
                self.cleanup();
            },
        }

        // Now we are ready to run experiments by pushing messages to the
        // gateway stream.
    }

    /// Kill the different components in order.
    pub fn cleanup(&mut self) {
        if self.nanovm.is_some() {
            debug!("Sending SIGINT to nano VM");
            match kill(Pid::from_raw(self.nanovm.as_mut().unwrap().id() as i32), Signal::SIGINT) {
                Ok(_) => {},
                Err(e) => error!("error sending SIGINT to nano VM: {e:?}"),
            }
        }

        if self.linuxd.is_some() {
            debug!("Sending SIGINT to linuxd");
            match kill(Pid::from_raw(self.linuxd.as_mut().unwrap().id() as i32), Signal::SIGINT) {
                Ok(_) => {},
                Err(e) => error!("error sending linuxd to nano VM: {e:?}"),
            }
        }

        // Remove the socket file
        match fs::remove_file(&self.linuxd_address) {
            Ok(_) => debug!("removed linuxd socket at: {}", &self.linuxd_address),
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                debug!("linuxd socket not found");
            },
            Err(e) => {
                // Non-fatal error, we are cleaning-up.
                error!(
                    "failed to delete linuxd socket file (file: {} - error: {e:?})",
                    &self.linuxd_address
                );
            },
        }

        // Gateway will be closed when dropped.
    }

    /// This function runs the cold-start experiment, where we measure the time to start linuxd,
    /// start a VM, and send a request to the new VM.
    pub fn run_cold_start(&mut self) -> Result<()> {
        // In the cold start experiment we cleanup and set-up at every iteration.
        self.cleanup();

        // Display a progress bar
        let num_iterations = 1e3 as usize;
        let pb = ProgressBar::new(num_iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        const DATA_SIZE: u32 = 10;
        let data = [7u8; DATA_SIZE as usize];

        // Payload we are sending over the wire
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        payload.extend_from_slice(&DATA_SIZE.to_le_bytes());
        payload.extend_from_slice(&data);

        // We don't use the send_to/recv_from gateway methods to prevent the data initialization
        // from being included in the cold-start time.
        let mut latencies: Vec<u128> = Vec::with_capacity(num_iterations);
        for iter in 0..num_iterations {
            self.linuxd_address = format!("/tmp/nanvix_coldstart_ubench_{iter}.socket");

            // Start the clock
            let start = Instant::now();
            self.setup();
            self.start();
            self.gateway.as_mut().unwrap().write_all(&payload)?;

            let mut response_payload: Vec<u8> = vec![0u8; mem::size_of::<u32>() + data.len()];
            self.gateway
                .as_mut()
                .unwrap()
                .read_exact(&mut response_payload)?;
            latencies.push(start.elapsed().as_micros());

            // Sanity-check the message to make sure is the same we sent.
            if response_payload != payload {
                error!("received payload does not match sent payload!");
                error!(" - sent: {payload:?}");
                error!(" - got: {response_payload:?}");
            }

            self.cleanup();
            pb.inc(1);

            // Need to give some time to clean-up
            thread::sleep(Duration::from_millis(10));
        }

        pb.finish();
        println!("First req: {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(num_iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(num_iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(num_iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    /// This function runs the warm start benchmark, where we measure the time to send a request
    /// into the VM once it has started executing.
    pub fn run_warm_start(&mut self) -> Result<()> {
        // Display a progress bar
        let num_iterations = 1e4 as usize;
        let pb = ProgressBar::new(num_iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        const DATA_SIZE: u32 = 10;
        let data = [7u8; DATA_SIZE as usize];

        // Payload we are sending over the wire
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        payload.extend_from_slice(&DATA_SIZE.to_le_bytes());
        payload.extend_from_slice(&data);

        // We don't use the send_to/recv_from gateway methods to prevent the data initialization
        // from being included in the cold-start time.
        let mut latencies: Vec<u128> = Vec::with_capacity(num_iterations);
        for _ in 0..num_iterations {
            let start = Instant::now();
            self.gateway.as_mut().unwrap().write_all(&payload)?;

            let mut response_payload: Vec<u8> = vec![0u8; mem::size_of::<u32>() + data.len()];
            self.gateway
                .as_mut()
                .unwrap()
                .read_exact(&mut response_payload)?;
            latencies.push(start.elapsed().as_micros());

            // Sanity-check the message to make sure is the same we sent.
            if response_payload != payload {
                error!("received payload does not match sent payload!");
                error!(" - sent: {payload:?}");
                error!(" - got: {response_payload:?}");
            }

            pb.inc(1);
        }

        pb.finish();
        println!("First req (includes nano VM boot time): {} us", latencies[0]);
        latencies.sort();
        println!("p50: {} us", latencies[(num_iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(num_iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(num_iterations as f32 * 0.99) as usize]);

        Ok(())
    }

    /// In this micro-benchmark we measure the time for a message to travel
    /// all the way from the VMM to the guest application and back. To achieve
    /// this, we connect the user VM to a gateway that emulates linuxd.
    pub fn run_warm_start_vmm(&mut self) -> Result<()> {
        // Clean-up deafult set-up.
        self.cleanup();

        // Display a progress bar.
        let num_iterations = 1e4 as usize;
        let pb = ProgressBar::new(num_iterations.try_into().unwrap());
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
                .expect("error creating progress bar")
                .progress_chars("#>-"),
        );
        pb.set_message("Benchmark progress:");

        const DATA_SIZE: u32 = 10;
        let data = [7u8; DATA_SIZE as usize];

        // Payload we are sending over the wire.
        let mut payload: Vec<u8> = Vec::with_capacity(mem::size_of::<u32>() + data.len());
        payload.extend_from_slice(&DATA_SIZE.to_le_bytes());
        payload.extend_from_slice(&data);
        let mut response_buf: [u8; ReadResponse::BUFFER_SIZE] = [0u8; ReadResponse::BUFFER_SIZE];
        response_buf[..payload.len()].copy_from_slice(&payload);

        let mut latencies: Vec<u128> = Vec::with_capacity(num_iterations);
        for _ in 0..num_iterations {
            // Clean-up the default set-up, and initialize a UNIX pair that is directly connected to the
            // VMM.
            let (mut input_stream, vmm_stream) = UnixStream::pair()?;

            // Spawn the VMM in a separate thread.
            let vmm_handle = std::thread::spawn(move || -> Result<()> {
                vmm_stream.set_nonblocking(true)?;
                let mut vmm: Vmm = Vmm::new(
                    config::kernel::MEMORY_SIZE,
                    format!("{}/bin/kernel.elf", get_proj_root()).as_str(),
                    Some(format!("{}/bin/echo-single-rust-nostd.elf", get_proj_root())),
                    None,
                    None,
                    Some(Gateway::new(syscomm::SocketStream::Unix(vmm_stream))),
                )?;
                debug!("VMM: returned from new!");

                match vmm.run()? {
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

            // Before starting the timer, we need to receive the ReadRequest from the user VM.
            let mut buf: [u8; config::kernel::IPC_MESSAGE_SIZE] =
                [0u8; config::kernel::IPC_MESSAGE_SIZE];
            input_stream.read_exact(&mut buf)?;
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
            input_stream.read_exact(&mut buf)?;
            latencies.push(start.elapsed().as_micros());

            // After receiving the WriteRequest, we need to acknowledge it by sending a WriteResponse.
            let write_response: Message = WriteResponse::build(tid, payload.len() as i32);
            input_stream.write_all(&write_response.to_bytes())?;

            // Wait for the VMM to exit.
            match vmm_handle.join() {
                Ok(_) => {},
                Err(_) => return Err(anyhow::anyhow!("Error running VMM")),
            }

            std::thread::sleep(std::time::Duration::from_millis(10));

            pb.inc(1);
        }

        pb.finish();
        latencies.sort();
        println!("p50: {} us", latencies[(num_iterations as f32 * 0.5) as usize]);
        println!("p95: {} us", latencies[(num_iterations as f32 * 0.95) as usize]);
        println!("p99: {} us", latencies[(num_iterations as f32 * 0.99) as usize]);

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

fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    {
        error!(
            "WARNING: the Nanvix benchmarks require compilation with RELEASE=yes and \
             LOG_LEVEL=panic"
        );
        return Ok(());
    }

    // Initialize logger, and make sure we print error logs.
    Logger::try_with_env_or_str("error")
        .expect("malformed RUST_LOG environment variable")
        .start()
        .expect("failed to initialize logger");

    let args: Args = Args::parse(std::env::args().collect())?;

    // Initialize HwLoc and pin main thread.
    let hwloc = args.hwloc();
    if hwloc.is_some() {
        hwloc::pin_main_thread(hwloc.clone().unwrap().get_client_core_str())?;
    }

    let mut benchmark = Benchmark {
        hwloc,
        flavour: args.benchmark(),
        gateway_address: GATEWAY_ADDRESS.to_string(),
        linuxd_address: NANVIX_LINUXD_UNIX_SOCKET.to_string(),
        linuxd: None,
        nanovm: None,
        gateway: None,
    };

    print!("Setting up {} benchmark...", benchmark.flavour);
    benchmark.setup();
    benchmark.start();
    println!("done!");

    let result = match benchmark.flavour {
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
                benchmark.run_cold_start()
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
                benchmark.run_warm_start()
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
        Err(e) => error!("error running benchmark: {e:?}"),
    }

    print!("Cleaning up...");
    benchmark.cleanup();
    println!("done!");

    Ok(())
}
