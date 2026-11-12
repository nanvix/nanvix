// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod vmm;

mod gateway;
mod socket_echo;
mod standalone;
mod standalone_socket;
mod standalone_syscall;
mod standalone_vfs;

//==================================================================================================
// Constants
//==================================================================================================

///
/// # Description
///
/// Sleep duration (in ms) to wait for the system to clean up after a benchmark run.
///
pub(crate) const CLEANUP_SLEEP_DURATION: u64 = 10;

///
/// # Description
///
/// Sleep duration (in ms) after the warmup echo, before timed iterations begin.
///
pub(crate) const WARMUP_SLEEP_DURATION: u64 = CLEANUP_SLEEP_DURATION;

///
/// # Description
///
/// Maximum number messages that can be queued in a channel.
///
pub(crate) const CHANNEL_CAPACITY: usize = 1024;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Prints a percentile latency table (p50, p95, p99) with the given header.
///
/// # Parameters
///
/// - `header`: Title of the table, printed at the start of its header row.
/// - `unit`: Unit of the latencies, printed in the column headers (e.g., `us`).
/// - `data`: Latency samples of each table row, keyed by row name. Samples are sorted in place.
///
pub(crate) fn print_latency_table(header: &str, unit: &str, data: &mut [(&str, Vec<u128>)]) {
    let p50_label: String = format!("p50 ({unit})");
    let p95_label: String = format!("p95 ({unit})");
    let p99_label: String = format!("p99 ({unit})");
    println!(
        "\n{:<22} {:>8} {:>10} {:>10} {:>10}",
        header, "Samples", p50_label, p95_label, p99_label
    );
    println!("{}", "-".repeat(62));

    for (name, latencies) in data.iter_mut() {
        latencies.sort();
        let len: usize = latencies.len();
        let p50: u128 = latencies[((len as f64 * 0.5) as usize).min(len - 1)];
        let p95: u128 = latencies[((len as f64 * 0.95) as usize).min(len - 1)];
        let p99: u128 = latencies[((len as f64 * 0.99) as usize).min(len - 1)];
        println!("{:<22} {:>8} {:>10} {:>10} {:>10}", name, len, p50, p95, p99);
    }
}
