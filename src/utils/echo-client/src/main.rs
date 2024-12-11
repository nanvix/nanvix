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

use self::args::Args;
use ::anyhow::Result;
use ::flexi_logger::Logger;
use ::serde_json::{
    json,
    Value,
};
use ::std::{
    env,
    sync::{
        atomic::AtomicUsize,
        Arc,
        Once,
    },
    thread,
    time::{
        Duration,
        Instant,
    },
};
use ::tokio::{
    io::{
        AsyncBufReadExt,
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
    sync::{
        mpsc,
        Mutex,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging system.
    initialize();

    // Parse and retrieve command-line arguments.
    let args: Args = Args::parse(env::args().collect())?;
    let nthreads: usize = args.nthreads();
    let frequency: u128 = args.frequency();
    let timeout: u64 = args.timeout();
    let sockaddr: String = args.server_sockaddr();
    let latencies: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::with_capacity(2 ^ 16)));

    let (stop_tx, stop_rx) = mpsc::channel(1);

    let sockaddr: String = sockaddr.clone();
    let http_request: Arc<Vec<u8>> = Arc::new(build_request());
    let latencies2: Arc<Mutex<Vec<u64>>> = latencies.clone();
    let thread = tokio::spawn(async move {
        client(latencies2, sockaddr, http_request, frequency, stop_rx).await
    });

    thread::sleep(Duration::from_secs(timeout));

    // Stop all threads.
    if let Err(e) = stop_tx.send(true).await {
        anyhow::bail!("failed to send stop signal: {}", e);
    }
    let nrequests = thread.await??;

    // Compute statistics from latencies.
    let latencies_guard = latencies.lock().await;
    let mut sorted_latencies: Vec<u64> = latencies_guard.clone();
    sorted_latencies.sort();

    let p50_index: usize = ((sorted_latencies.len() * 50) / 100).max(1) - 1;
    let p99_index: usize = ((sorted_latencies.len() * 99) / 100).max(1) - 1;

    let p50 = sorted_latencies[p50_index];
    let p99 = sorted_latencies[p99_index];

    println!("{:?},{:?},{:?},{:?},{:?},{:?}", nthreads, frequency, timeout, nrequests, p50, p99);

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

async fn client(
    latencies: Arc<Mutex<Vec<u64>>>,
    sockaddr: String,
    http_request: Arc<Vec<u8>>,
    frequency: u128,
    mut stop_rx: mpsc::Receiver<bool>,
) -> Result<usize, anyhow::Error> {
    // Send first request.
    let mut stop_sending: bool = false;
    let mut last_sent: Instant = std::time::Instant::now();
    let nrequests: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    loop {
        if stop_sending {
            debug!("stopping client...");
            for handle in handles {
                if let Err(e) = handle.await? {
                    error!("failed to join handle: {}", e);
                }
            }
            debug!("stopped!");
            return Ok(nrequests.load(std::sync::atomic::Ordering::Relaxed));
        } else if last_sent.elapsed().as_nanos() >= frequency {
            let http_request2: Arc<Vec<u8>> = http_request.clone();
            let sockaddr2: String = sockaddr.clone();
            let requests2 = nrequests.clone();
            let latencies2 = latencies.clone();

            let handle = tokio::spawn(async move {
                let now = std::time::Instant::now();
                let mut stream: TcpStream = TcpStream::connect(sockaddr2).await?;
                debug!("connected to server");
                stream.write_all(&http_request2).await?;

                // Read a line
                loop {
                    let mut response = vec![0u8; 1024];
                    match stream.read(&mut response).await {
                        Ok(n) => {
                            if n == 0 {
                                anyhow::bail!("Connection closed by server");
                            }
                            // Try to read the response
                            let reader = tokio::io::BufReader::new(&response[..n] as &[u8]);
                            let mut lines = reader.lines();
                            let mut done = false;
                            while let Some(line) = lines.next_line().await? {
                                if line.is_empty() {
                                    let elapsed: u128 = now.elapsed().as_nanos();

                                    stream.shutdown().await?;
                                    debug!("elapsed: {} ns", elapsed);
                                    latencies2.lock().await.push(elapsed as u64);
                                    requests2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    done = true;
                                    break;
                                }
                            }

                            if done {
                                break;
                            }
                        },
                        Err(e) => {
                            anyhow::bail!("failed to read from socket: {}", e);
                        },
                    }
                }

                debug!("disconnected from server");

                Ok(())
            });

            handles.push(handle);

            last_sent = std::time::Instant::now();
        }

        if !stop_sending && stop_rx.try_recv().is_ok() {
            stop_sending = true;
        }
    }
}

fn build_request() -> Vec<u8> {
    let json_obj: Value = json!({
        "clientid": 1,
        "program": "bin/echo.elf",
        "args": ["hello, world"]
    });

    format!(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        json_obj.to_string().len(),
        json_obj
    )
    .as_bytes()
    .to_vec()
}
