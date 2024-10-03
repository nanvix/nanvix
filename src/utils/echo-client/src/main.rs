// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Modules
//==================================================================================================

mod args;

//==================================================================================================
// Imports
//==================================================================================================

// Must come first.
#[macro_use]
extern crate log;

extern crate core_affinity;

use self::args::Args;
use anyhow::Result;
use core_affinity::CoreId;
use flexi_logger::Logger;
use histogram::{
    AtomicHistogram,
    Histogram,
};
use serde_json::{
    json,
    Value,
};
use std::{
    collections::VecDeque,
    env,
    io::{
        BufRead,
        Read,
        Write,
    },
    sync::{
        mpsc,
        Arc,
        Barrier,
        Once,
    },
    thread,
    thread::JoinHandle,
    time::{
        Duration,
        Instant,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn main() -> Result<()> {
    // Initialize logging system.
    initialize();

    // Parse and retrieve command-line arguments.
    let args: Args = Args::parse(env::args().collect())?;
    let nthreads: usize = args.nthreads();
    let frequency: u128 = args.frequency();
    let timeout: u64 = args.timeout();
    let sockaddr: String = args.server_sockaddr();

    let mut next_pid: usize = 0;
    let stats: Arc<AtomicHistogram> = Arc::new(AtomicHistogram::new(7, 64)?);

    let mut stream: std::net::TcpStream = match std::net::TcpStream::connect(sockaddr.clone()) {
        Ok(s) => s,
        Err(e) => {
            anyhow::bail!("Failed to connect: {}", e);
        },
    };

    let barrier: Arc<Barrier> = Arc::new(Barrier::new(nthreads + 1));

    let core_ids: Vec<CoreId> = core_affinity::get_core_ids().unwrap();

    if !core_affinity::set_for_current(core_ids[0]) {
        anyhow::bail!("failed to set core affinity");
    }

    let core_ids: Vec<CoreId> = core_ids[1..]
        .iter()
        .take(nthreads)
        .copied()
        .collect::<Vec<_>>();

    type ThreadResult = Result<usize, anyhow::Error>;

    let threads: Vec<(JoinHandle<ThreadResult>, mpsc::Sender<bool>)> = core_ids
        .into_iter()
        .map(|id| {
            let (stop_tx, stop_rx): (mpsc::Sender<bool>, mpsc::Receiver<bool>) =
                mpsc::channel::<bool>();
            let sockaddr: String = sockaddr.clone();
            let stats: Arc<AtomicHistogram> = stats.clone();
            let barrier: Arc<Barrier> = Arc::clone(&barrier);
            // Create ping request.
            let http_request: Vec<u8> = build_request(next_pid, 1, "ping".as_bytes());
            next_pid += 1;
            (
                thread::spawn(move || {
                    client(id, sockaddr, http_request, stats, barrier, frequency, stop_rx)
                }),
                stop_tx,
            )
        })
        .collect::<Vec<_>>();

    barrier.wait();

    thread::sleep(Duration::from_secs(timeout));

    // Stop all threads.
    for (_, stop_tx) in threads.iter() {
        stop_tx.send(true).unwrap();
    }

    // Wait for the thread to finish
    let mut block: usize = 0;
    for (i, (handle, _stop_tx)) in threads.into_iter().enumerate() {
        trace!("waiting on thread {}", i);
        match handle.join() {
            Ok(Ok(b)) => {
                if b > block {
                    block = b;
                }
            },
            Ok(Err(e)) => {
                anyhow::bail!("thread failed: {}", e);
            },
            Err(_) => {
                anyhow::bail!("failed to join thread");
            },
        }
    }

    // Create a JSON object with fields "source", "destination", and "input"
    let exit_request = build_request(next_pid, 1, "exit".as_bytes());

    stream.set_read_timeout(None)?;

    // Send exit request.
    send_request(&mut stream, &exit_request)?;
    try_read_response(&mut stream)?;
    trace!("exit request sent");

    let stats: Histogram = stats.load();

    let p50: u64 = stats.percentile(0.50)?.unwrap().start();
    let p90: u64 = stats.percentile(0.90)?.unwrap().start();
    let p95: u64 = stats.percentile(0.95)?.unwrap().start();
    let p99: u64 = stats.percentile(0.99)?.unwrap().start();

    println!(
        "{:?},{:?},{:?},{:?},{:?},{:?},{:?},{:?}",
        nthreads, frequency, timeout, p50, p90, p95, p99, block
    );

    Ok(())
}

fn send_request(stream: &mut std::net::TcpStream, bytes: &[u8]) -> Result<()> {
    match stream.write_all(bytes) {
        Ok(_) => Ok(()),
        Err(e) => {
            anyhow::bail!("Failed to write to stream: {}", e);
        },
    }
}

fn try_read_response(stream: &mut std::net::TcpStream) -> Result<()> {
    let mut buf_reader = std::io::BufReader::new(stream);
    for line in buf_reader.by_ref().lines() {
        let line = line?;
        if line.is_empty() {
            break;
        }
    }
    Ok(())
}

///
/// # Description
///
/// Initializes the logger.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn initialize() {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        Logger::try_with_env()
            .expect("malformed RUST_LOG environment variable")
            .start()
            .expect("failed to initialize logger");
    });
}

fn client(
    id: CoreId,
    sockaddr: String,
    http_request: Vec<u8>,
    stats: Arc<AtomicHistogram>,
    barrier: Arc<Barrier>,
    frequency: u128,
    stop_rx: mpsc::Receiver<bool>,
) -> Result<usize, anyhow::Error> {
    if !core_affinity::set_for_current(id) {
        anyhow::bail!("failed to set core affinity");
    }

    trace!("thread in core {:?}", id);

    // Open a TCP connection to the server.
    let mut stream: std::net::TcpStream = match std::net::TcpStream::connect(sockaddr) {
        Ok(s) => s,
        Err(e) => {
            anyhow::bail!("Failed to connect: {}", e);
        },
    };
    stream.set_read_timeout(Some(Duration::from_nanos(1)))?;

    // Wait on the barrier
    barrier.wait();

    // Send first request.
    let mut inflight_requests: usize = 0;
    let mut stop_sending: bool = false;
    let mut last_sent: Instant = std::time::Instant::now();

    let mut timers: VecDeque<Instant> = VecDeque::new();
    let mut block: usize = 0;

    let mut response: [u8; 1024] = [0u8; 1024];

    loop {
        if !stop_sending && last_sent.elapsed().as_nanos() >= frequency {
            last_sent = std::time::Instant::now();
            inflight_requests += 1;
            send_request(&mut stream, &http_request)?;
            timers.push_back(last_sent);
        }

        if stop_sending && inflight_requests > 0 {
            break Ok(block);
        }

        // Attempt to read the response from the server
        match stream.read(&mut response) {
            Ok(n) => {
                if n == 0 {
                    anyhow::bail!("Connection closed by server");
                }

                // Try to read the response
                for line in response[..n].lines() {
                    match line {
                        Ok(line) => {
                            if line.is_empty() {
                                if let Some(now) = timers.pop_front() {
                                    let elapsed: u128 = now.elapsed().as_nanos();
                                    stats.increment(elapsed as u64)?;
                                    inflight_requests -= 1;
                                    break;
                                } else {
                                    anyhow::bail!("received response without sending request");
                                }
                            }
                        },
                        Err(e) => {
                            anyhow::bail!("Failed to read from stream: {}", e);
                        },
                    }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                block += 1;
                continue;
            },

            Err(e) => {
                anyhow::bail!("Failed to read from stream: {}", e);
            },
        }

        if !stop_sending && stop_rx.try_recv().is_ok() {
            stop_sending = true;
        }
    }
}

fn build_request(source: usize, destination: usize, payload: &[u8]) -> Vec<u8> {
    let json_obj: Value = json!({
        "source": source,
        "destination": destination,
        "payload": payload
    });

    format!(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json_obj.to_string().len(),
        json_obj
    )
    .as_bytes()
    .to_vec()
}
