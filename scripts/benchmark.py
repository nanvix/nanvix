# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# ======================================================================
# Imports
# ======================================================================

import argparse
import itertools
import os
import pathlib
import re
import subprocess

# ======================================================================
# Constants
# ======================================================================

HYPERLIGHT_MACHINE_TYPE = "hyperlight"
MICROVM_MACHINE_TYPE = "microvm"
NA = "NA"
NANVIX_BENCH_ELF = "nanvix-bench.elf"
PERCENTILES = ["p50", "p95", "p99"]
ROUND_TRIP_SIZES = ["32B", "64B", "128B", "256B", "512B", "1KiB", "4KiB"]
X86_64_ARCH = "X64"

# ======================================================================
# Benchmark Names
# ======================================================================

L2_SUFFIX = "-l2"
BOOT_TIME_BENCH = "boot-time"
COLD_START_BENCH = "cold-start"
COLD_START_L2_BENCH = COLD_START_BENCH + L2_SUFFIX
COLD_START_UVM_BENCH = "cold-start-uvm"
CONCURRENT_BENCH = "concurrent"
CONCURRENT_L2_BENCH = CONCURRENT_BENCH + L2_SUFFIX
ECHO_BREAKDOWN_BENCH = "echo-breakdown"
ECHO_BREAKDOWN_L2_BENCH = ECHO_BREAKDOWN_BENCH + L2_SUFFIX
ROUND_TRIP_LATENCY_BENCH = "round-trip-latency"
WARM_START_BENCH = "warm-start"
WARM_START_L2_BENCH = WARM_START_BENCH + L2_SUFFIX
WARM_START_VMM_BENCH = "warm-start-vmm"

# ======================================================================
# Benchmark Constants
# ======================================================================

# How many user VMs do we spawn in parallel in the CONCURRENT* benchmarks.
NUM_CONCURRENT_VMS = 100

# ======================================================================
# Helper Functions
# ======================================================================


def wait_for_tcp_cleanup(max_wait_seconds=70):
    """
    Wait for TCP connections in TIME_WAIT state to clear.

    This function polls the system to check if there are lingering TCP connections
    in TIME_WAIT state (typically from previous L2 benchmark runs) and waits until
    they are cleared or the timeout is reached.

    # Parameters

    - `max_wait_seconds`: Maximum time to wait in seconds (default: 70).

    # Returns

    True if connections cleared, False if timeout reached.
    """
    import time

    print(f"[TCP-CLEANUP] Starting TCP cleanup check (max_wait={max_wait_seconds}s)")
    start_time: float = time.time()
    poll_interval: int = 2  # Check every 2 seconds.

    while (time.time() - start_time) < max_wait_seconds:
        try:
            # Count connections in TIME_WAIT state on port 9999 (nanvixd default port).
            print("[TCP-CLEANUP] Checking TIME_WAIT connections on port 9999...")
            result: subprocess.CompletedProcess = subprocess.run(
                ["ss", "-tan", "state", "time-wait", "sport", "9999"],
                capture_output=True,
                text=True,
                check=False,
            )

            if result.returncode == 0:
                # Count non-header lines.
                lines: list = [
                    line for line in result.stdout.splitlines() if line.strip()
                ]
                # First line is header, so subtract 1.
                time_wait_count: int = max(0, len(lines) - 1)

                print(f"[TCP-CLEANUP] Found {time_wait_count} TIME_WAIT connection(s)")

                if time_wait_count == 0:
                    print("[TCP-CLEANUP] All TCP connections cleared successfully.")
                    return True

                elapsed: int = int(time.time() - start_time)
                print(
                    f"Waiting for {time_wait_count} TIME_WAIT connection(s) "
                    f"to clear... ({elapsed}s elapsed)"
                )

        except Exception as e:
            print(f"[TCP-CLEANUP] ERROR: Failed to check TCP connection state: {e}")
            print(f"[TCP-CLEANUP] Falling back to fixed wait of {max_wait_seconds}s")
            # Fall back to fixed wait if we can't check.
            time.sleep(max_wait_seconds)
            return False

        time.sleep(poll_interval)

    print(
        f"[TCP-CLEANUP] WARNING: Timeout reached after {max_wait_seconds}s, "
        f"some connections may still be in TIME_WAIT"
    )
    return False


def cleanup_stale_netns():
    """
    Cleans up any stale Nanvix network namespaces left from previous runs.

    This function removes network namespaces that match the Nanvix naming pattern
    (nvxns-*) to prevent resource conflicts when running L2 benchmarks.
    """
    print("[NETNS-CLEANUP] Starting network namespace cleanup...")
    try:
        # List all network namespaces and filter for Nanvix ones.
        result = subprocess.run(
            ["sudo", "ip", "netns", "list"], capture_output=True, text=True, check=False
        )

        if result.returncode != 0:
            # If command fails, just continue (user may not have permissions).
            print(
                f"[NETNS-CLEANUP] WARNING: Failed to list namespaces "
                f"(exit code {result.returncode})"
            )
            return

        # Extract Nanvix network namespace names (nvxns-*).
        import re
        import time

        netns_list = re.findall(r"nvxns-\d+", result.stdout)
        print(f"[NETNS-CLEANUP] Found {len(netns_list)} Nanvix namespace(s)")

        if netns_list:
            print(
                f"[NETNS-CLEANUP] Cleaning up {len(netns_list)} stale network namespace(s)..."
            )
            for ns in netns_list:
                # Extract the namespace ID from the name.
                ns_id = ns.replace("nvxns-", "")

                # Delete veth pair first (host side).
                veth_name = f"nvxgw-h-{ns_id}"
                veth_result = subprocess.run(
                    ["sudo", "ip", "link", "del", veth_name],
                    capture_output=True,
                    check=False,
                )
                if veth_result.returncode != 0:
                    print(f"[NETNS-CLEANUP] WARNING: Failed to delete veth {veth_name}")

                # Delete the namespace.
                ns_result = subprocess.run(
                    ["sudo", "ip", "netns", "del", ns], capture_output=True, check=False
                )
                if ns_result.returncode != 0:
                    print(f"[NETNS-CLEANUP] WARNING: Failed to delete namespace {ns}")

            # Give the system time to fully release network resources.
            time.sleep(1)
            print("[NETNS-CLEANUP] Cleanup completed successfully.")
        else:
            print("[NETNS-CLEANUP] No stale namespaces found.")
    except Exception as e:
        # Non-fatal: just log and continue.
        print(
            f"[NETNS-CLEANUP] ERROR: Failed to clean up stale network namespaces: {e}"
        )
        import traceback

        print(f"[NETNS-CLEANUP] Traceback:\n{traceback.format_exc()}")


def gen_filename_for_benchmark(benchmark, machine_type, arch):
    benchmark = benchmark.replace("-", "_")
    return f"nanvix_bench_{benchmark}_{machine_type}_{arch}.csv"


def filter_benchmark_stdout(benchmark, raw_stdout):
    """
    Helper method to convert a benchmark's raw stdout into a formatted CSV
    string.
    """
    # All these benchmarks report the results in the form of:
    # p50: <val> us
    # p95: <val> us
    # p99: <val> us
    if benchmark in [
        BOOT_TIME_BENCH,
        COLD_START_BENCH,
        COLD_START_L2_BENCH,
        COLD_START_UVM_BENCH,
        CONCURRENT_BENCH,
        CONCURRENT_L2_BENCH,
        WARM_START_BENCH,
        WARM_START_L2_BENCH,
        WARM_START_VMM_BENCH,
    ]:
        pattern = re.compile(
            r"^\s*(p50|p95|p99)\s*:\s*([0-9]+)\s*us\b", re.IGNORECASE | re.MULTILINE
        )
        values = {}
        for k, v in pattern.findall(raw_stdout):
            values[k.lower()] = int(v)

        # Ensure all three are present
        missing = [percentile for percentile in PERCENTILES if percentile not in values]
        if missing:
            print(
                f"ERROR: missing percentile values for benchmark '{benchmark}': {missing}"
            )
            raise ValueError("Missing percentile values in benchmark results")

        filtered_stdout = ",".join(PERCENTILES)
        filtered_stdout += "\n"
        filtered_stdout += ",".join(
            [str(values[percentile]) for percentile in PERCENTILES]
        )
    elif benchmark == ROUND_TRIP_LATENCY_BENCH:
        filtered_stdout = "size," + ",".join(PERCENTILES)
        actual_sizes = []
        for line in raw_stdout.splitlines():
            if not line.strip():
                continue

            # Skip header lines.
            if line.lstrip().lower().startswith("size"):
                continue

            # Split line on tabs (one or more).
            parts = re.split(r"\t+", line.strip())
            if len(parts) < 4:
                print(f"ERROR: malformatted output from '{benchmark}' benchmark")
                raise ValueError("Malformed output")

            size = parts[0].rstrip(":").replace(" ", "")
            actual_sizes.append(size)
            p50, p95, p99 = parts[1:4]
            filtered_stdout += "\n" + ",".join([size, p50, p95, p99])

        # Sanity check we have a value for each size.
        if actual_sizes != ROUND_TRIP_SIZES:
            print(f"ERROR: did not collect expected sizes in '{benchmark}'")
            print(f"ERROR: expected: {ROUND_TRIP_SIZES} - got: {actual_sizes}")
            raise ValueError("Not expected values.")

    elif benchmark.startswith(ECHO_BREAKDOWN_BENCH):
        filtered_stdout = raw_stdout

    else:
        print(f"ERROR: unrecognized benchmark '{benchmark}'")
        raise ValueError("Unrecognized benchmark")

    return filtered_stdout


def read_benchmark_values_from_file(benchmark, file_path, percentile=None):
    """
    Helper method to read the benchmark results from a file.
    """
    # All these benchmarks have the same CSV format.
    if benchmark in [
        BOOT_TIME_BENCH,
        COLD_START_BENCH,
        COLD_START_L2_BENCH,
        COLD_START_UVM_BENCH,
        CONCURRENT_BENCH,
        CONCURRENT_L2_BENCH,
        WARM_START_BENCH,
        WARM_START_L2_BENCH,
        WARM_START_VMM_BENCH,
    ]:
        try:
            with open(file_path, "r") as fh:
                result_line = fh.readlines()[1].split(",")
                result_dict = {}

                for percentile, result in zip(PERCENTILES, result_line):
                    result_dict[percentile] = result
        except Exception:
            result_dict = {}
            for percentile in PERCENTILES:
                result_dict[percentile] = NA
    elif benchmark == ROUND_TRIP_LATENCY_BENCH:
        line_idx = PERCENTILES.index(percentile) + 1
        try:
            with open(file_path, "r") as f:
                lines = f.readlines()
                lines = [line.strip() for line in lines][1:]

                result_dict = {}
                for msg_size, line in zip(ROUND_TRIP_SIZES, lines):
                    line = line.split(",")
                    if msg_size != line[0]:
                        print(
                            f"ERROR: unexpected message size, expected: {msg_size} - got: {line[0]}"
                        )
                        raise ValueError("Unexpected message size")
                    result_dict[msg_size] = line[line_idx]
        except Exception:
            result_dict = {}
            for msg_size in ROUND_TRIP_SIZES:
                result_dict[msg_size] = NA
    else:
        print(f"ERROR: unrecognized benchmark '{benchmark}'")
        raise ValueError("Unrecognized benchmark")

    return result_dict


def get_table_rows_for_benchmark(
    benchmark, machine_arch_combinations, dev_dir, target_dir, percentile=None
):
    rows = {}

    for machine, arch in machine_arch_combinations:
        results_file = gen_filename_for_benchmark(benchmark, machine, arch)

        dev_path = os.path.join(dev_dir, results_file)
        dev_vals = read_benchmark_values_from_file(benchmark, dev_path, percentile)

        tgt_path = os.path.join(target_dir, results_file)
        tgt_vals = read_benchmark_values_from_file(benchmark, tgt_path, percentile)

        must_add_keys = len(rows) == 0
        for key, dev_val in dev_vals.items():
            if key not in tgt_vals:
                print(
                    f"ERROR: key mismatch generating rows for '{benchmark}' benchmark"
                )
                print(f"ERROR: dev values: {dev_vals} - target values: {tgt_vals}")
                raise ValueError("Key mismatch loading rows.")
            tgt_val = tgt_vals[key]

            if must_add_keys:
                rows[key] = []

            # Calculate delta
            if tgt_val == NA or dev_val == NA:
                delta_str = NA
            else:
                pct = float((int(tgt_val) / int(dev_val)) * 100)
                if pct > 100:
                    delta_str = "+{:.1f}%".format(pct - 100.0)
                else:
                    delta_str = "-{:.1f}%".format(100.0 - pct)

            rows[key].append(dev_val)
            rows[key].append(tgt_val)
            rows[key].append(delta_str)

    return rows


def make_header(benchmark, table_width):
    title = f"{benchmark} (us)"
    total_width = table_width
    padding = (
        total_width - len(title) - 2
    ) // 2  # minus 2 for leading and trailing '='
    left = "=" * padding
    right = "=" * (total_width - len(left) - len(title) - 2)
    return f"{left} {title} {right}\n"


def generate_benchmark_table(
    dev_dir, target_dir, benchmark, machines, archs, percentile=None
):
    """
        Generate a table summarizing the benchmark results, and comparing them
        to the results of the current `dev` branch.

        Each table has the following structure:

    =================================== boot-time (us) ====================================
    |       |            microvm (X64)             |           hyperlight (X64)           |
    |       |    dev     |   target   |     Δ      |    dev     |   target   |     Δ      |
    ...
    =======================================================================================

        where the rows are benchmark-dependent.
    """
    # Calculate table dimensions
    first_col_width = 7  # "| p50 "
    sub_col_width = 12  # Width for each sub-column (dev, target, delta)
    machine_col_width = sub_col_width * 3  # 3 sub-columns per machine

    # Number of column groups is cartesian product of machines and archs
    groups = list(itertools.product(machines, archs))
    groups_count = len(groups)

    # Calculate total table width correctly:
    # 2 for leading and trailing '|', plus first column width, plus for each group
    # 3 sub-columns (machine_col_width) and 3 internal separators between them.
    # This yields the exact length of any data/sub-header row.
    table_width = 2 + first_col_width + (groups_count * (machine_col_width + 3))

    # Create header for this benchmark
    if benchmark == ROUND_TRIP_LATENCY_BENCH and percentile is not None:
        header = make_header(benchmark + f"-{percentile}", table_width)
    else:
        header = make_header(benchmark, table_width)

    # Create table structure
    table_lines = []

    # Header row with machine names
    machine_header_parts = [f" {'':^{first_col_width-2}} "]
    for machine, arch in groups:
        machine_name = f"{machine} ({arch})"
        # Each machine spans 3 sub-columns + 2 separators
        machine_span = sub_col_width * 3 + 2
        machine_header_parts.append(f"{machine_name:^{machine_span}}")
    machine_header_line = "|" + "|".join(machine_header_parts) + "|"
    table_lines.append(machine_header_line)

    # Sub-header row with dev/target/Δ columns
    sub_header_parts = [f" {'':^{first_col_width-2}} "]
    for _ in groups:
        sub_header_parts.extend(
            [
                f"{'dev':^{sub_col_width}}",
                f"{'target':^{sub_col_width}}",
                f"{'Δ':^{sub_col_width}}",
            ]
        )
    sub_header_line = "|" + "|".join(sub_header_parts) + "|"
    table_lines.append(sub_header_line)

    rows = get_table_rows_for_benchmark(
        benchmark, groups, dev_dir, target_dir, percentile
    )
    for row_label, row_values in rows.items():
        row_parts = [f" {row_label:^{first_col_width-2}} "]

        for i in range(0, len(row_values), 3):
            # Add each sub-column separately
            row_parts.extend(
                [
                    f"{row_values[i]:^{sub_col_width}}",
                    f"{row_values[i+1]:^{sub_col_width}}",
                    f"{row_values[i+2]:^{sub_col_width}}",
                ]
            )

        # Join the row parts with | separators
        row_line = "|" + "|".join(row_parts) + "|"
        table_lines.append(row_line)

    # Add footer
    footer_str = "=" * table_width

    # Combine everything for this benchmark
    table_str = "\n".join(table_lines)
    output = header + table_str + "\n" + footer_str + "\n"

    return output


# ======================================================================
# Entrypoint Functions
# ======================================================================


def ci_summary(args):
    """
    Generate a markdown summary to include in PRs as a comment.
    """
    print(
        f"Generating CI summary (benchmarks={args.benchmarks}, "
        f"machine_types={args.machine_types}, archs={args.archs})"
    )

    benchmarks = args.benchmarks.split(",")
    machines = args.machine_types.split(",")
    archs = args.archs.split(",")

    # General-purpose benchmarks that we put in similar tables.
    bench_summary = "```"
    for benchmark in benchmarks:
        if benchmark in [
            BOOT_TIME_BENCH,
            COLD_START_BENCH,
            COLD_START_L2_BENCH,
            COLD_START_UVM_BENCH,
            CONCURRENT_BENCH,
            CONCURRENT_L2_BENCH,
            WARM_START_BENCH,
            WARM_START_L2_BENCH,
            WARM_START_VMM_BENCH,
        ]:
            bench_summary += "\n" + generate_benchmark_table(
                args.dev_dir, args.target_dir, benchmark, machines, archs
            )
        elif benchmark == ROUND_TRIP_LATENCY_BENCH:
            for percentile in PERCENTILES:
                bench_summary += "\n" + generate_benchmark_table(
                    args.dev_dir,
                    args.target_dir,
                    benchmark,
                    machines,
                    archs,
                    percentile,
                )
        elif benchmark.startswith(ECHO_BREAKDOWN_BENCH):
            # We handle the echo-breakdown benchmarks separately.
            continue
        else:
            print(f"ERROR: unrecognized benchmark '{benchmark}'")
            raise ValueError("Unrecognized benchmark")
    bench_summary += "```" + "\n"

    # Echo breakdown benchmarks that we put in a collapsed section.
    echo_breakdown_summary = None
    echo_breakdown_benchmarks = [ECHO_BREAKDOWN_BENCH, ECHO_BREAKDOWN_L2_BENCH]
    filtered_benchmarks = list(
        filter(lambda b: b in echo_breakdown_benchmarks, benchmarks)
    )
    if len(filtered_benchmarks) > 0:
        # For the echo-breakdown benchmarks we dump whatever the benchmark
        # outputs in a collapsed section.
        echo_breakdown_summary = (
            "<details>\n<summary>Data-Path Breakdown</summary>\n\n```"
        )
        table_width = 91

        for benchmark in filtered_benchmarks:
            for machine, arch in list(itertools.product(machines, archs)):
                echo_breakdown_summary += "\n" + make_header(
                    f"{benchmark} {machine}", table_width
                )

                file_name = gen_filename_for_benchmark(benchmark, machine, arch)
                with open(os.path.join(args.target_dir, file_name), "r") as fh:
                    echo_breakdown_summary += fh.read()
                echo_breakdown_summary += "=" * table_width + "\n"

        echo_breakdown_summary += "\n```\n</details>\n"

    if echo_breakdown_summary is not None:
        bench_summary += "\n" + echo_breakdown_summary

    with open(args.output_file, "w") as fh:
        fh.write(bench_summary)


def run_benchmark(args):
    """
    Run a single benchmark using nanvix-bench
    """
    print(
        f"[BENCHMARK] Running '{args.benchmark}' benchmark "
        f"(machine={args.machine_type}, arch={X86_64_ARCH})"
    )
    print(
        f"[BENCHMARK] Configuration: iterations={args.iterations}, "
        f"hwloc={args.hwloc}"
    )
    print(
        f"[BENCHMARK] Paths: bin_dir={args.bin_dir}, "
        f"toolchain_bin_dir={args.toolchain_bin_dir}"
    )

    # Before running L2 benchmarks, wait for TCP connections from previous runs to clear.
    # This is critical when L2 benchmarks run after non-L2 benchmarks in sequence.
    if args.benchmark.endswith(L2_SUFFIX):
        print(
            "[BENCHMARK] This is an L2 benchmark, checking for lingering TCP connections..."
        )
        cleanup_success = wait_for_tcp_cleanup()
        print(
            f"[BENCHMARK] TCP cleanup result: {'success' if cleanup_success else 'timeout/failure'}"
        )

    # Clean up any stale network namespaces before running all benchmarks.
    # This prevents resource conflicts from previous runs, especially when running
    # non-L2 benchmarks after L2 benchmarks in a sequence.
    print("[BENCHMARK] Cleaning up stale network namespaces...")
    cleanup_stale_netns()

    # The concurrent benchmark takes slightly different command-line arguments than the other
    # benchmarks. It does not take a `-hwloc` file, and instead of `-iterations` it takes
    # a number of concurrent user VMs.
    is_concurrent_bench = args.benchmark.startswith(CONCURRENT_BENCH)
    print(f"[BENCHMARK] Is concurrent benchmark: {is_concurrent_bench}")

    nanvix_bench_cmd = [
        os.path.join(args.bin_dir, NANVIX_BENCH_ELF),
        f"-benchmark {args.benchmark}",
        f"-hwloc {args.hwloc}" if not is_concurrent_bench else "",
        (
            f"-iterations {args.iterations}"
            if not is_concurrent_bench
            else f"-num-concurrent-vms {NUM_CONCURRENT_VMS}"
        ),
        f"-toolchain-bin-dir {args.toolchain_bin_dir}",
    ]
    nanvix_bench_cmd = " ".join(nanvix_bench_cmd)
    print(f"[BENCHMARK] Executing command: {nanvix_bench_cmd}")

    if args.output_dir is not None:
        print(f"[BENCHMARK] Output will be saved to: {args.output_dir}")
        output_dir = pathlib.Path(args.output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)
        output_file = os.path.join(
            output_dir,
            gen_filename_for_benchmark(args.benchmark, args.machine_type, X86_64_ARCH),
        )

        # Run benchmark and capture raw stdout/stderr.
        print("[BENCHMARK] Starting benchmark execution...")
        result = subprocess.run(nanvix_bench_cmd, shell=True, capture_output=True)
        print(f"[BENCHMARK] Benchmark completed with exit code: {result.returncode}")
        if result.returncode != 0:
            print(
                f"[BENCHMARK] ERROR: benchmark '{args.benchmark}' failed "
                f"with exit code {result.returncode}"
            )
            print("[BENCHMARK] STDOUT:")
            print(result.stdout.decode("utf-8"))
            print("[BENCHMARK] STDERR:")
            print(result.stderr.decode("utf-8"))

            # Additional diagnostics for L2 benchmarks.
            if args.benchmark.endswith(L2_SUFFIX):
                print("[BENCHMARK] Running post-failure network diagnostics...")
                diag_result = subprocess.run(
                    ["ss", "-tan", "state", "time-wait", "sport", "9999"],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                print(
                    f"[BENCHMARK] TIME_WAIT connections on port 9999:\n{diag_result.stdout}"
                )

                netns_result = subprocess.run(
                    ["sudo", "ip", "netns", "list"],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                print(f"[BENCHMARK] Active network namespaces:\n{netns_result.stdout}")
            raise RuntimeError(
                f"Benchmark '{args.benchmark}' failed with exit code {result.returncode}"
            )

        print("[BENCHMARK] Processing benchmark output...")
        raw_stdout = result.stdout.decode("utf-8")
        print(f"[BENCHMARK] Raw stdout length: {len(raw_stdout)} bytes")
        filtered_stdout = filter_benchmark_stdout(args.benchmark, raw_stdout)
        print(f"[BENCHMARK] Filtered stdout length: {len(filtered_stdout)} bytes")
        print(f"[BENCHMARK] Writing results to: {output_file}")
        with open(output_file, "w") as fh:
            fh.write(filtered_stdout)
        print("[BENCHMARK] Results written successfully.")
    else:
        print("[BENCHMARK] Running benchmark without capturing output...")
        subprocess.run(nanvix_bench_cmd, shell=True, check=True)

    # After L2 benchmarks, wait for TCP connections in TIME_WAIT to clear.
    # L2 benchmarks create many TCP connections that linger in TIME_WAIT state,
    # which can cause connection issues for subsequent benchmarks.
    if args.benchmark.endswith(L2_SUFFIX):
        print("[BENCHMARK] Post-benchmark: checking for lingering TCP connections...")
        cleanup_success = wait_for_tcp_cleanup()
        result_str = "success" if cleanup_success else "timeout/failure"
        print(f"[BENCHMARK] Post-benchmark TCP cleanup result: {result_str}")

    print(f"[BENCHMARK] Benchmark '{args.benchmark}' completed successfully.")


def copy_results(args):
    benchmarks = args.benchmarks.split(",")
    machines = args.machine_types.split(",")
    archs = args.archs.split(",")
    groups = list(itertools.product(benchmarks, machines, archs))

    for benchmark, machine, arch in groups:
        filename = gen_filename_for_benchmark(benchmark, machine, arch)
        cmd = f"cp {args.source_dir}/{filename} {args.target_dir}/{filename}"
        print(cmd)
        # Tolerate failures in the cp command, indicating a missing benchmark.
        subprocess.run(cmd, shell=True, check=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Run and plot the Nanvix performance benchmarks"
    )
    # Initialize sub-parsers.
    sub_parser = parser.add_subparsers(dest="cmd", required=True)
    run_parser = sub_parser.add_parser("run", help="Run a performance benchmark")
    ci_summary_parser = sub_parser.add_parser(
        "ci-summary", help="Generate a summary of the benchmark results"
    )
    copy_results_parser = sub_parser.add_parser(
        "copy-results", help="Copy results to a target directory"
    )

    # Command-line arguments for the run command.
    run_parser.add_argument(
        "--benchmark",
        required=True,
        help="Name of the benchmark to run",
    )
    # Eventually make the machine type something we can pass to nanvix-bench
    # too.
    run_parser.add_argument(
        "--machine-type",
        required=True,
        choices=[MICROVM_MACHINE_TYPE, HYPERLIGHT_MACHINE_TYPE],
        help="Type of machine to run the benchmarks on",
    )
    # Make the --hwloc file a mandatory argument for the wrapper script, to
    # make sure that it is set.
    run_parser.add_argument(
        "--hwloc",
        required=True,
        help="Path to file with the hardware locality information",
    )
    run_parser.add_argument(
        "--bin-dir",
        help="Directory where to find nanvix-bench.elf",
        default="./bin",
    )
    run_parser.add_argument(
        "--toolchain-bin-dir",
        help="Toolchain binary directory",
        default="./toolchain/bin",
    )
    run_parser.add_argument(
        "--iterations",
        type=int,
        default=100,
        help="Path to file with the hardware locality information",
    )
    run_parser.add_argument(
        "--output-dir",
        help="Directory where to store the benchmark results. If not set prints to stdout",
    )
    run_parser.set_defaults(func=run_benchmark)

    # Command-line arguments for the ci-summary command.
    ci_summary_parser.add_argument(
        "--dev-dir", required=True, help="Directory for the results from the dev branch"
    )
    ci_summary_parser.add_argument(
        "--target-dir",
        required=True,
        help="Directory for the results from the current branch",
    )
    ci_summary_parser.add_argument(
        "--benchmarks",
        required=True,
        help="Comma-separated list of benchmarks to aggregate",
    )
    ci_summary_parser.add_argument(
        "--machine-types",
        required=True,
        help="Comma-separated list of machines types benchmarked",
    )
    ci_summary_parser.add_argument(
        "--archs",
        required=True,
        help="Comma-separated list of architectures benchmarked",
    )
    ci_summary_parser.add_argument(
        "--output-file", required=True, help="File to output the benchmark summary"
    )
    ci_summary_parser.set_defaults(func=ci_summary)

    # Command-line arguments for the ci-summary command.
    copy_results_parser.add_argument(
        "--source-dir", required=True, help="Directory with results"
    )
    copy_results_parser.add_argument(
        "--target-dir", required=True, help="Target directory to copy results"
    )
    copy_results_parser.add_argument(
        "--benchmarks",
        required=True,
        help="Comma-separated list of benchmarks to aggregate",
    )
    copy_results_parser.add_argument(
        "--machine-types",
        required=True,
        help="Comma-separated list of machines types benchmarked",
    )
    copy_results_parser.add_argument(
        "--archs",
        required=True,
        help="Comma-separated list of architectures benchmarked",
    )
    copy_results_parser.set_defaults(func=copy_results)

    args = parser.parse_args()
    # Dispatch the arguments to the selected top-level command.
    args.func(args)
