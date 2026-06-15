// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! White-box warm-start benchmark for the OpenVMM-based Nanvix guest.
//!
//! Boots the echo guest once with an in-process [`ChannelGuestIo`] endpoint
//! (no operating-system pipe) and times many host -> guest -> host stdio round
//! trips. This isolates the latency *in and out of the VMM* over its IKC path,
//! giving parity with the native Nanvix `warm-start-vmm` micro-benchmark. It is
//! driven by `bench-compare.py`, which parses the `<metric>: <N> us` lines this
//! binary prints to stdout.

use ::nanvixd_vmm::{
    build_guest_image,
    init_logging,
    io::ChannelGuestIo,
    open_console,
    vmm,
    DEFAULT_MEM_SIZE,
};
use ::std::{
    path::PathBuf,
    process::ExitCode,
    time::Instant,
};

/// Environment variable that overrides the host-side log filter.
const LOG_ENV_VAR: &str = "NANVIXD_VMM_LOG";

/// The echo guest used for the round-trip measurement.
const ECHO_PROGRAM: &str = "echo-rust-nostd.initrd";

/// Parsed benchmark configuration.
struct Args {
    /// Directory containing `kernel.elf` and the echo guest.
    bin_dir: PathBuf,
    /// Number of timed round trips.
    iterations: usize,
    /// Number of untimed warmup round trips.
    warmup: usize,
    /// Payload size, in bytes, per round trip.
    payload: usize,
}

impl Args {
    /// Parses arguments with the defaults `bench-compare.py` relies on.
    fn parse(mut args: impl Iterator<Item = String>) -> ::anyhow::Result<Self> {
        let mut parsed = Args {
            bin_dir: PathBuf::from("./bin"),
            iterations: 1000,
            warmup: 100,
            payload: 1,
        };
        let _ = args.next();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-bin-dir" => parsed.bin_dir = PathBuf::from(value(&mut args, "-bin-dir")?),
                "-iterations" => parsed.iterations = value(&mut args, "-iterations")?.parse()?,
                "-warmup" => parsed.warmup = value(&mut args, "-warmup")?.parse()?,
                "-payload" => parsed.payload = value(&mut args, "-payload")?.parse()?,
                other => anyhow::bail!("unknown argument: {other}"),
            }
        }
        if parsed.iterations == 0 {
            anyhow::bail!("-iterations must be greater than zero");
        }
        if parsed.payload == 0 {
            anyhow::bail!("-payload must be greater than zero");
        }
        Ok(parsed)
    }
}

/// Returns the value following an option, or an error if it is missing.
fn value(args: &mut impl Iterator<Item = String>, option: &str) -> ::anyhow::Result<String> {
    args.next()
        .ok_or_else(|| anyhow::anyhow!("missing value for {option}"))
}

fn main() -> ExitCode {
    init_logging(LOG_ENV_VAR);
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ::log::error!("nanvixd-vmm-warmstart failed: {e:?}");
            ExitCode::FAILURE
        },
    }
}

fn run() -> ::anyhow::Result<()> {
    let args: Args = Args::parse(::std::env::args())?;

    let image = build_guest_image(
        &args.bin_dir,
        Some(args.bin_dir.join(ECHO_PROGRAM)),
        None,
        None,
        None,
        DEFAULT_MEM_SIZE,
    );
    // Keep the guest console off stdout so it never mixes with the report.
    let console = open_console(None)?;

    let (guest_io, mut handle) = ChannelGuestIo::pair();
    let vm_thread = ::std::thread::Builder::new()
        .name("nanvixd-vmm-warmstart-guest".to_string())
        .spawn(move || {
            ::pal_async::DefaultPool::run_with(move |driver| async move {
                vmm::run(driver, image, Box::new(guest_io), console, None, false).await
            })
        })?;

    let payload: Vec<u8> = vec![b'a'; args.payload];

    // Warmup round trips (untimed): also lets the guest finish booting before we
    // start measuring.
    for _ in 0..args.warmup {
        handle.send(&payload);
        if !handle.read_exact(payload.len()) {
            anyhow::bail!("guest closed stdout during warmup");
        }
    }

    // Timed round trips.
    let mut latencies_us: Vec<u64> = Vec::with_capacity(args.iterations);
    for _ in 0..args.iterations {
        let start = Instant::now();
        handle.send(&payload);
        if !handle.read_exact(payload.len()) {
            anyhow::bail!("guest closed stdout during measurement");
        }
        latencies_us.push(start.elapsed().as_micros() as u64);
    }

    // Signal EOF so the guest exits, then reap the VM thread.
    handle.close_input();
    let _ = vm_thread.join();

    report(&mut latencies_us);
    Ok(())
}

/// Prints the latency distribution in the `<metric>: <N> us` format the
/// comparison harness parses.
fn report(latencies_us: &mut [u64]) {
    latencies_us.sort_unstable();
    let n: usize = latencies_us.len();
    let pick = |p: f64| -> u64 {
        let index: usize = ((p * (n as f64 - 1.0)).round() as usize).min(n - 1);
        latencies_us[index]
    };
    let sum: u128 = latencies_us.iter().map(|&v| u128::from(v)).sum();
    let mean: u64 = (sum / n as u128) as u64;

    println!("warm-start-vmm (in-process IKC round trip)");
    println!("iterations: {n}");
    println!("min: {} us", latencies_us[0]);
    println!("p50: {} us", pick(0.50));
    println!("p95: {} us", pick(0.95));
    println!("p99: {} us", pick(0.99));
    println!("mean: {mean} us");
}
