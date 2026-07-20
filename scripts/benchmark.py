# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# ======================================================================
# Imports
# ======================================================================

import argparse
import collections
import csv
import io
import itertools
import math
import os
import pathlib
import platform
import re
import signal
import subprocess
import sys
from typing import Optional

# ======================================================================
# Constants
# ======================================================================

MICROVM_MACHINE_TYPE = "microvm"
IS_WINDOWS = platform.system() == "Windows"
NA = "NA"
NANVIX_BENCH_BINARY = "nanvix-bench.exe" if IS_WINDOWS else "nanvix-bench.elf"
PERCENTILES = ["p50", "p95", "p99"]
WARM_START_VMM_SIZES = ["32B", "1KiB", "4KiB", "8KiB", "16KiB", "32KiB", "64KiB"]
WARM_START_SOCKET_SIZES = ["32B", "1KiB", "4KiB", "8KiB", "16KiB"]
WARM_START_GATEWAY_SIZES = ["32B", "64B", "128B", "256B", "512B", "1KiB", "4KiB"]
WARM_START_VMM_MIN_PAYLOAD_SIZE = 4
X86_64_ARCH = "X64"

# ======================================================================
# Benchmark Names
# ======================================================================

BOOT_TIME_BENCH = "boot-time"
COLD_START_BENCH = "cold-start"
COLD_START_UVM_BENCH = "cold-start-uvm"
SNAPSHOT_RESTORE_BENCH = "snapshot-restore"
VFS_BENCH = "vfs-bench"
WARM_START_GATEWAY_BENCH = "warm-start-gateway"
WARM_START_VMM_BENCH = "warm-start-vmm"
WARM_START_SOCKET_BENCH = "warm-start-socket"
PAYLOAD_SIZE_BENCHMARKS = [
    WARM_START_GATEWAY_BENCH,
    WARM_START_VMM_BENCH,
    WARM_START_SOCKET_BENCH,
]
SIZE_SWEEP_BENCHMARKS = {
    WARM_START_GATEWAY_BENCH: WARM_START_GATEWAY_SIZES,
    WARM_START_VMM_BENCH: WARM_START_VMM_SIZES,
    WARM_START_SOCKET_BENCH: WARM_START_SOCKET_SIZES,
}
SIZE_AWARE_BENCHMARKS = SIZE_SWEEP_BENCHMARKS

# Start distinct histories for the standalone implementations while preserving the public
# benchmark names used by the command-line interface.
RESULT_FILE_BENCHMARK_SUFFIX = "-standalone"

# ======================================================================
# Benchmark Constants
# ======================================================================

# Per-benchmark timeout in seconds. Long-running benchmarks (e.g. size sweeps)
# can take many minutes on resource-constrained GitHub runners. A 45-minute
# timeout provides headroom while preventing indefinite hangs.
BENCHMARK_TIMEOUT_SECS = 45 * 60

# Benchmarks that report simple p50/p95/p99 percentile values.
PERCENTILE_BENCHMARKS = [
    BOOT_TIME_BENCH,
    COLD_START_BENCH,
    COLD_START_UVM_BENCH,
    SNAPSHOT_RESTORE_BENCH,
]

# ======================================================================
# Helper Functions
# ======================================================================


def _positive_int(value: str) -> int:
    """Argparse type: require a positive integer (> 0)."""
    try:
        ivalue = int(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"invalid int value: '{value}'")
    if ivalue <= 0:
        raise argparse.ArgumentTypeError(f"must be a positive integer, got {ivalue}")
    return ivalue


def _non_negative_float(value: str) -> float:
    """Argparse type: require a finite, non-negative float (>= 0)."""
    try:
        fvalue = float(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"invalid float value: '{value}'")
    if math.isnan(fvalue) or math.isinf(fvalue) or fvalue < 0:
        raise argparse.ArgumentTypeError(
            f"must be a finite non-negative float, got {value}"
        )
    return fvalue


def _split_csv_arg(value: str) -> list[str]:
    """Split a comma-separated string, stripping whitespace and dropping empties."""
    return [item.strip() for item in value.split(",") if item.strip()]


def gen_filename_for_benchmark(benchmark: str, machine_type: str, arch: str) -> str:
    """Generate the CSV filename for a given benchmark, machine type, and architecture."""
    benchmark = f"{benchmark}{RESULT_FILE_BENCHMARK_SUFFIX}"
    benchmark = benchmark.replace("-", "_")
    return f"nanvix_bench_{benchmark}_{machine_type}_{arch}.csv"


def _format_size_key(size_bytes: int) -> str:
    """Format a byte size like nanvix-bench size-sweep output after normalization."""
    if size_bytes >= 1024 and size_bytes % 1024 == 0:
        return f"{size_bytes // 1024}KiB"
    return f"{size_bytes}B"


def _parse_percentile_values(benchmark: str, raw_stdout: str) -> dict[str, int]:
    """Extract p50/p95/p99 values from scalar percentile benchmark output."""
    # The snapshot-restore benchmark prints multiple percentile blocks
    # (Cold-start, Snapshot restore, Post-restore execution). Restrict
    # parsing to the headline "Snapshot restore (...)" section so the
    # CSV continues to track the same metric across releases. For all
    # other percentile benchmarks the full stdout is scanned.
    if benchmark == SNAPSHOT_RESTORE_BENCH:
        section_match = re.search(
            r"^Snapshot restore \(.*?\):\s*\n(?P<body>(?:[ \t]+.*\n?)+)",
            raw_stdout,
            re.MULTILINE,
        )
        if section_match is None:
            print(
                "ERROR: missing 'Snapshot restore (...)' section in "
                f"benchmark '{benchmark}' output"
            )
            raise ValueError("Missing 'Snapshot restore' section")
        search_scope = section_match.group("body")
    else:
        search_scope = raw_stdout

    pattern = re.compile(
        r"^\s*(p50|p95|p99)\s*:\s*([0-9]+)\s*us\b", re.IGNORECASE | re.MULTILINE
    )
    values: dict[str, int] = {}
    for k, v in pattern.findall(search_scope):
        values[k.lower()] = int(v)

    # Ensure all three percentiles are present.
    missing = [p for p in PERCENTILES if p not in values]
    if missing:
        print(
            f"ERROR: missing percentile values for benchmark '{benchmark}': {missing}"
        )
        raise ValueError("Missing percentile values in benchmark results")

    return values


def filter_benchmark_stdout(
    benchmark: str,
    raw_stdout: str,
    commit: str,
    expected_sizes: Optional[list[str]] = None,
) -> str:
    """
    Convert a benchmark's raw stdout into a formatted CSV string.

    Every CSV includes a ``commit`` column as the first field so that results
    can be accumulated over time in a single history file.

    # Parameters

    - ``benchmark``: Name of the benchmark.
    - ``raw_stdout``: Raw stdout captured from the benchmark binary.
    - ``commit``: Commit SHA to tag this result with.

    # Returns

    A CSV string (header + data rows) with the benchmark metrics.
    """
    if benchmark in PERCENTILE_BENCHMARKS:
        values = _parse_percentile_values(benchmark, raw_stdout)
        header = "commit," + ",".join(PERCENTILES)
        data = commit + "," + ",".join(str(values[p]) for p in PERCENTILES)
        filtered_stdout = header + "\n" + data

    elif benchmark in SIZE_SWEEP_BENCHMARKS:
        header = "commit,size," + ",".join(PERCENTILES)
        data_lines = []
        actual_sizes = []
        if expected_sizes is None:
            expected_sizes = SIZE_SWEEP_BENCHMARKS[benchmark]
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
            data_lines.append(",".join([commit, size, p50, p95, p99]))

        # Sanity check we have a value for each size.
        if actual_sizes != expected_sizes:
            print(f"ERROR: did not collect expected sizes in '{benchmark}'")
            print(f"ERROR: expected: {expected_sizes} - got: {actual_sizes}")
            raise ValueError("Not expected values.")

        filtered_stdout = header + "\n" + "\n".join(data_lines)

    elif benchmark == VFS_BENCH:
        # VFS benchmark prints two tables (raw operations and paired
        # decomposition). Each table has a header line followed by a dash
        # separator and then data rows.  Format per data row (Rust
        # `println!("{:<22} {:>8} {:>10} {:>10} {:>10}", ...)`):
        #   <operation>  <samples>  <p50>  <p95>  <p99>
        columns = ["commit", "section", "operation", "samples", "p50", "p95", "p99"]
        buf = io.StringIO()
        writer = csv.writer(buf, lineterminator="\n")
        writer.writerow(columns)

        current_section = ""
        row_count = 0
        for line in raw_stdout.splitlines():
            stripped = line.strip()
            if not stripped:
                continue

            # Detect section header (the header row starts with a
            # left-aligned section name followed by "Samples").
            if "Samples" in stripped and "p50" in stripped:
                # The section name is everything before the first column
                # header keyword.
                current_section = stripped.split("Samples")[0].strip()
                continue

            # Skip separator lines.
            if stripped.startswith("-"):
                continue

            # Parse data rows: operation (22 chars) then numeric columns.
            # Only accept rows after a section header has been detected to
            # avoid capturing preamble lines (e.g. FAT image statistics).
            parts = stripped.split()
            if current_section and len(parts) >= 4:
                operation = parts[0]
                samples = parts[1]
                p50 = parts[2]
                p95 = parts[3]
                p99 = parts[4] if len(parts) > 4 else parts[3]
                writer.writerow(
                    [commit, current_section, operation, samples, p50, p95, p99]
                )
                row_count += 1

        if row_count == 0:
            print("ERROR: no data parsed from vfs-bench output")
            raise ValueError("No data in vfs-bench output")

        filtered_stdout = buf.getvalue().rstrip("\r\n")

    else:
        print(f"ERROR: unrecognized benchmark '{benchmark}'")
        raise ValueError("Unrecognized benchmark")

    return filtered_stdout


def read_benchmark_values_from_file(
    benchmark: str, file_path: str, percentile: Optional[str] = None
) -> dict[str, str]:
    """
    Read the latest commit's benchmark values from a CSV file.

    Works for both single-run files (one data row) and history files
    (multiple data rows). Always returns the values from the last commit
    present in the file.

    # Parameters

    - ``benchmark``: Name of the benchmark.
    - ``file_path``: Path to the CSV file.
    - ``percentile``: For size-aware benchmarks, which percentile column to read.

    # Returns

    A dict mapping metric keys to their string values.
    """
    if benchmark in PERCENTILE_BENCHMARKS:
        try:
            with open(file_path, "r") as fh:
                lines = [line.strip() for line in fh.readlines() if line.strip()]

            if len(lines) < 2:
                raise ValueError("No data rows")

            # Last data row format: commit,p50,p95,p99
            last_row = lines[-1].split(",")
            result_dict = {}
            for p_name, val in zip(PERCENTILES, last_row[1:]):
                result_dict[p_name] = val
        except (FileNotFoundError, ValueError, IndexError) as exc:
            print(f"WARNING: could not read {file_path}: {exc}")
            result_dict = {p: NA for p in PERCENTILES}

    elif benchmark in SIZE_AWARE_BENCHMARKS:
        try:
            with open(file_path, "r") as f:
                lines = [line.strip() for line in f.readlines() if line.strip()]

            if len(lines) < 2:
                raise ValueError("No data rows")

            if percentile is None:
                percentile = PERCENTILES[0]

            expected_sizes: list[str] = SIZE_AWARE_BENCHMARKS[benchmark]
            # Find the last commit present in the file.
            last_commit = lines[-1].split(",")[0]

            # Column layout: commit=0, size=1, p50=2, p95=3, p99=4
            col_idx = PERCENTILES.index(percentile) + 2

            result_dict = {}
            for line in lines[1:]:
                parts = line.split(",")
                if parts[0] == last_commit:
                    size = parts[1]
                    result_dict[size] = parts[col_idx]

            # Fill in any missing sizes with NA.
            for size in expected_sizes:
                if size not in result_dict:
                    result_dict[size] = NA
        except (FileNotFoundError, ValueError, IndexError) as exc:
            print(f"WARNING: could not read {file_path}: {exc}")
            result_dict = {s: NA for s in SIZE_AWARE_BENCHMARKS[benchmark]}

    elif benchmark == VFS_BENCH:
        # VFS benchmark CSV: commit,section,operation,samples,p50,p95,p99
        # Return the latest commit's rows keyed by "section/operation".
        try:
            with open(file_path, "r") as fh:
                lines = [line.strip() for line in fh.readlines() if line.strip()]
            if len(lines) < 2:
                raise ValueError("No data rows")
            last_commit = lines[-1].split(",")[0]
            result_dict = {}
            for line in lines[1:]:
                parts = line.split(",")
                if parts[0] == last_commit:
                    key = f"{parts[1]}/{parts[2]}"
                    result_dict[key] = f"{parts[4]}/{parts[5]}/{parts[6]}"
        except (FileNotFoundError, ValueError, IndexError) as exc:
            print(f"WARNING: could not read {file_path}: {exc}")
            result_dict = {}

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

    ============ boot-time (us) ============
    |       |      microvm (X64)           |
    |       |  dev  | target |      Δ      |
    ...
    ========================================

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
    if benchmark in SIZE_AWARE_BENCHMARKS and percentile is not None:
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

    benchmarks = _split_csv_arg(args.benchmarks)
    machines = _split_csv_arg(args.machine_types)
    archs = _split_csv_arg(args.archs)

    # General-purpose benchmarks that we put in similar tables.
    bench_summary = "```"
    for benchmark in benchmarks:
        if benchmark in PERCENTILE_BENCHMARKS:
            bench_summary += "\n" + generate_benchmark_table(
                args.dev_dir, args.target_dir, benchmark, machines, archs
            )
        elif benchmark in SIZE_AWARE_BENCHMARKS:
            for percentile in PERCENTILES:
                bench_summary += "\n" + generate_benchmark_table(
                    args.dev_dir,
                    args.target_dir,
                    benchmark,
                    machines,
                    archs,
                    percentile,
                )
        elif benchmark == VFS_BENCH:
            # VFS benchmark results are reported in a collapsed section.
            continue
        else:
            print(f"ERROR: unrecognized benchmark '{benchmark}'")
            raise ValueError("Unrecognized benchmark")
    bench_summary += "```" + "\n"

    # VFS benchmark collapsed section.
    if VFS_BENCH in benchmarks:
        vfs_summary = "<details>\n<summary>VFS Benchmark</summary>\n\n```"
        table_width = 91
        for machine, arch in list(itertools.product(machines, archs)):
            file_name = gen_filename_for_benchmark(VFS_BENCH, machine, arch)
            file_path = os.path.join(args.target_dir, file_name)
            vfs_summary += "\n" + make_header(f"{VFS_BENCH} {machine}", table_width)
            if os.path.exists(file_path):
                try:
                    with open(file_path, "r") as fh:
                        reader = csv.DictReader(fh)
                        current_section = ""
                        for row in reader:
                            if row["section"] != current_section:
                                current_section = row["section"]
                                vfs_summary += (
                                    f"\n{current_section:<22} "
                                    f"{'Samples':>8} "
                                    f"{'p50 (us)':>10} "
                                    f"{'p95 (us)':>10} "
                                    f"{'p99 (us)':>10}\n"
                                )
                                vfs_summary += "-" * 62 + "\n"
                            vfs_summary += (
                                f"{row['operation']:<22} "
                                f"{row['samples']:>8} "
                                f"{row['p50']:>10} "
                                f"{row['p95']:>10} "
                                f"{row['p99']:>10}\n"
                            )
                except Exception as exc:
                    vfs_summary += f"  (could not read results: {exc})\n"
            else:
                vfs_summary += "  (no results available)\n"
            vfs_summary += "=" * table_width + "\n"
        vfs_summary += "\n```\n</details>\n"
        bench_summary += "\n" + vfs_summary

    with open(args.output_file, "w") as fh:
        fh.write(bench_summary)


def _read_baseline_moving_avg(
    benchmark: str, file_path: str, window: int = 20
) -> Optional[float]:
    """
    Read the last ``window`` p50 values from a baseline CSV and return their average.

    Works for scalar percentile benchmarks using the ``commit,p50,p95,p99`` schema.
    Returns ``None`` when the file is missing or contains no valid p50 values.

    # Parameters

    - ``benchmark``: Name of the benchmark.
    - ``file_path``: Path to the baseline CSV file.
    - ``window``: Number of most-recent data rows to average.
    """
    if benchmark not in PERCENTILE_BENCHMARKS:
        return None

    try:
        with open(file_path, "r") as fh:
            reader = iter(fh)
            # Skip header.
            header = next(reader, None)
            if header is None:
                print(f"WARNING: no data rows in baseline: {file_path}")
                return None
            p50_idx = 1
            tail: collections.deque[str] = collections.deque(maxlen=window)
            for line in reader:
                stripped = line.strip()
                if not stripped:
                    continue
                tail.append(stripped)
    except FileNotFoundError:
        print(f"WARNING: baseline file not found: {file_path}")
        return None

    if not tail:
        print(f"WARNING: no data rows in baseline: {file_path}")
        return None

    p50_values: list[float] = []
    for row in tail:
        parts = row.split(",")
        if len(parts) <= p50_idx:
            continue
        try:
            p50_values.append(float(parts[p50_idx]))
        except (ValueError, TypeError):
            continue

    if not p50_values:
        print(f"WARNING: no valid p50 values in baseline: {file_path}")
        return None

    return sum(p50_values) / len(p50_values)


def ci_gate(args) -> int:
    """
    Check benchmark results for performance regressions against baseline.

    Compares the p50 value of each benchmark CSV in target-dir against the
    moving average of the last N baseline p50 values in dev-dir. Exits
    non-zero if any benchmark regresses beyond the configured threshold.

    Scalar percentile benchmarks are checked. Size-sweep benchmarks and benchmarks with missing
    baseline or missing results are skipped with a warning.
    """
    threshold = args.regression_threshold
    window = args.baseline_window
    benchmarks = _split_csv_arg(args.benchmarks)
    machines = _split_csv_arg(args.machine_types)
    archs = _split_csv_arg(args.archs)

    regressions = []
    checked = 0

    for benchmark in benchmarks:
        if benchmark not in PERCENTILE_BENCHMARKS:
            print(f"SKIP: '{benchmark}' is not a regression-gated benchmark")
            continue

        for machine, arch in itertools.product(machines, archs):
            csv_name = gen_filename_for_benchmark(benchmark, machine, arch)
            dev_path = os.path.join(args.dev_dir, csv_name)
            tgt_path = os.path.join(args.target_dir, csv_name)

            # Read baseline moving average of p50 values.
            dev_avg = _read_baseline_moving_avg(benchmark, dev_path, window)
            if dev_avg is None:
                print(f"SKIP: no baseline for {benchmark} ({machine}/{arch})")
                continue
            if dev_avg == 0:
                print(
                    f"WARNING: zero baseline avg for {benchmark} "
                    f"({machine}/{arch}), skipping"
                )
                continue

            # Read current p50.
            tgt_vals = read_benchmark_values_from_file(benchmark, tgt_path)
            if tgt_vals.get("p50", NA) == NA:
                print(f"SKIP: no results for {benchmark} ({machine}/{arch})")
                continue

            try:
                tgt_p50 = float(tgt_vals["p50"])
            except (ValueError, TypeError) as e:
                print(
                    f"SKIP: invalid p50 value for {benchmark} ({machine}/{arch}): {e}"
                )
                continue

            checked += 1

            delta_pct = (tgt_p50 - dev_avg) / dev_avg * 100

            if delta_pct > threshold:
                regressions.append(
                    {
                        "benchmark": benchmark,
                        "machine": machine,
                        "arch": arch,
                        "dev_avg": round(dev_avg, 1),
                        "tgt_p50": tgt_p50,
                        "delta_pct": round(delta_pct, 1),
                    }
                )
                print(
                    f"REGRESSION: {benchmark} ({machine}/{arch}): "
                    f"p50 {tgt_p50} vs baseline avg {round(dev_avg, 1)} "
                    f"(+{round(delta_pct, 1)}%, threshold: {threshold}%)"
                )
            else:
                status = f"+{delta_pct:.1f}%" if delta_pct > 0 else f"{delta_pct:.1f}%"
                print(
                    f"OK: {benchmark} ({machine}/{arch}): "
                    f"p50 {tgt_p50} vs baseline avg {round(dev_avg, 1)} ({status})"
                )

    print(
        f"\nChecked {checked} benchmark(s), "
        f"found {len(regressions)} regression(s) "
        f"(threshold: >{threshold}% vs {window}-point moving avg)."
    )

    if regressions:
        print(
            f"\nFAILED: {len(regressions)} benchmark(s) exceeded "
            f"the {threshold}% regression threshold."
        )
        return 1

    print("PASSED: No regressions detected.")
    return 0


def _kill_process_tree(proc: subprocess.Popen) -> None:
    """Forcefully terminate a process and all its descendants.

    On Unix, benchmarks run in their own session (``start_new_session=True``),
    so killing the process group (PGID == PID) reaps all grandchildren.

    On Windows, ``taskkill /F /T`` is used to terminate the entire process
    tree rooted at the child PID.
    """
    try:
        if IS_WINDOWS:
            subprocess.run(
                ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        else:
            os.killpg(proc.pid, signal.SIGKILL)
    except OSError:
        pass  # Process (group) may already be gone.


def _run_with_timeout(
    cmd: str,
    *,
    timeout: int,
    capture_output: bool = False,
    check: bool = False,
) -> subprocess.CompletedProcess:
    """Run *cmd* in a new session, killing the entire process tree on timeout."""
    kwargs: dict = {
        "shell": True,
    }
    if IS_WINDOWS:
        kwargs["creationflags"] = subprocess.CREATE_NEW_PROCESS_GROUP
    else:
        kwargs["start_new_session"] = True
    if capture_output:
        kwargs["stdout"] = subprocess.PIPE
        kwargs["stderr"] = subprocess.PIPE

    proc = subprocess.Popen(cmd, **kwargs)
    try:
        stdout, stderr = proc.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        _kill_process_tree(proc)
        stdout, stderr = proc.communicate()
        raise subprocess.TimeoutExpired(
            proc.args, timeout, output=stdout, stderr=stderr
        )

    result = subprocess.CompletedProcess(proc.args, proc.returncode, stdout, stderr)
    if check and proc.returncode != 0:
        raise subprocess.CalledProcessError(proc.returncode, proc.args, stdout, stderr)
    return result


def run_benchmark(args):
    """
    Run a single benchmark using nanvix-bench
    """
    print(
        f"[BENCHMARK] Running '{args.benchmark}' benchmark "
        f"(machine={args.machine_type}, arch={X86_64_ARCH})"
    )
    # Normalize paths so that Unix-style "./" prefixes are converted to
    # platform-native form (cmd.exe does not understand "./").
    args.bin_dir = os.path.normpath(args.bin_dir)

    print(f"[BENCHMARK] Paths: bin_dir={args.bin_dir}")

    # Resolve the commit SHA used to tag this benchmark result.
    commit: str = args.commit
    if commit is None:
        git_result: subprocess.CompletedProcess = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=False,
        )
        if git_result.returncode != 0:
            print("ERROR: --commit not provided and 'git rev-parse HEAD' failed.")
            raise RuntimeError(
                "Cannot determine commit SHA. Pass --commit explicitly "
                "or run from inside a git repository."
            )
        commit = git_result.stdout.strip()
    print(f"[BENCHMARK] Commit: {commit}")

    payload_size = args.payload_size
    print(
        f"[BENCHMARK] Configuration: iterations={args.iterations}, "
        f"hwloc={args.hwloc}, payload_size={payload_size}"
    )
    if payload_size is not None and args.benchmark not in PAYLOAD_SIZE_BENCHMARKS:
        print(
            f"ERROR: --payload-size is not supported for benchmark '{args.benchmark}'."
        )
        raise ValueError("Unsupported payload-size benchmark")
    if (
        payload_size is not None
        and args.benchmark == WARM_START_VMM_BENCH
        and payload_size < WARM_START_VMM_MIN_PAYLOAD_SIZE
    ):
        print(
            "ERROR: --payload-size for warm-start-vmm must be at least "
            f"{WARM_START_VMM_MIN_PAYLOAD_SIZE} bytes because it includes the "
            "length prefix."
        )
        raise ValueError("Invalid warm-start-vmm payload size")

    nanvix_bench_cmd = [
        os.path.join(args.bin_dir, NANVIX_BENCH_BINARY),
        f"-benchmark {args.benchmark}",
        f"-hwloc {args.hwloc}" if args.hwloc else "",
        f"-iterations {args.iterations}",
        (
            f"-payload-size {payload_size}"
            if payload_size is not None and args.benchmark in PAYLOAD_SIZE_BENCHMARKS
            else ""
        ),
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
        try:
            result = _run_with_timeout(
                nanvix_bench_cmd,
                timeout=BENCHMARK_TIMEOUT_SECS,
                capture_output=True,
            )
        except subprocess.TimeoutExpired as e:
            print(
                f"[BENCHMARK] ERROR: benchmark '{args.benchmark}' timed out "
                f"after {BENCHMARK_TIMEOUT_SECS}s"
            )
            if e.stdout:
                print("[BENCHMARK] Partial STDOUT:")
                print(e.stdout.decode("utf-8", errors="replace"))
            if e.stderr:
                print("[BENCHMARK] Partial STDERR:")
                print(e.stderr.decode("utf-8", errors="replace"))
            raise RuntimeError(
                f"Benchmark '{args.benchmark}' timed out after "
                f"{BENCHMARK_TIMEOUT_SECS}s"
            )
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

            raise RuntimeError(
                f"Benchmark '{args.benchmark}' failed with exit code {result.returncode}"
            )

        print("[BENCHMARK] Processing benchmark output...")
        raw_stdout = result.stdout.decode("utf-8")
        raw_stderr = result.stderr.decode("utf-8", errors="replace")
        print(f"[BENCHMARK] Raw stdout length: {len(raw_stdout)} bytes")
        expected_sizes = None
        if args.benchmark in SIZE_AWARE_BENCHMARKS and payload_size is not None:
            expected_sizes = [_format_size_key(payload_size)]
        filtered_stdout = filter_benchmark_stdout(
            args.benchmark, raw_stdout, commit, expected_sizes
        )
        print(f"[BENCHMARK] Filtered stdout length: {len(filtered_stdout)} bytes")
        print(f"[BENCHMARK] Writing results to: {output_file}")
        with open(output_file, "w") as fh:
            fh.write(filtered_stdout)
        print("[BENCHMARK] Results written successfully.")

        # Save stderr alongside stdout for PERF_TIMINGS analysis.
        # When the binary is built with PROFILER=yes (profile-time feature),
        # stderr contains one PERF_TIMINGS:{json} line per iteration with
        # per-phase microsecond timings. This data enables distribution
        # analysis and regression detection via analyze-results.py.
        stderr_file = output_file.replace(".csv", "-stderr.txt")
        if raw_stderr:
            perf_lines = []
            for line in raw_stderr.splitlines():
                stripped = line.lstrip()
                if stripped.startswith("PERF_TIMINGS:"):
                    perf_lines.append(stripped)
                    continue
                # Handle lines where PERF_TIMINGS is embedded after other output.
                marker_index = line.find("PERF_TIMINGS:")
                if marker_index != -1:
                    perf_lines.append(line[marker_index:])
            if perf_lines:
                with open(stderr_file, "w") as fh:
                    fh.write("\n".join(perf_lines) + "\n")
                print(
                    f"[BENCHMARK] Saved {len(perf_lines)} PERF_TIMINGS records "
                    f"to {stderr_file}"
                )
    else:
        print("[BENCHMARK] Running benchmark without capturing output...")
        try:
            _run_with_timeout(
                nanvix_bench_cmd,
                timeout=BENCHMARK_TIMEOUT_SECS,
                check=True,
            )
        except subprocess.TimeoutExpired:
            print(
                f"[BENCHMARK] ERROR: benchmark '{args.benchmark}' timed out "
                f"after {BENCHMARK_TIMEOUT_SECS}s"
            )
            raise RuntimeError(
                f"Benchmark '{args.benchmark}' timed out after "
                f"{BENCHMARK_TIMEOUT_SECS}s"
            )

    print(f"[BENCHMARK] Benchmark '{args.benchmark}' completed successfully.")


def persist_results(args: argparse.Namespace) -> None:
    """
    Append current run results to history CSVs in the target directory.

    Each history CSV accumulates rows over time. New data rows from the
    source file are appended to the target file, preserving the header.
    If the target file does not yet exist it is created with the header
    from the source file.  Rows whose commit hash already exists in the
    target are skipped to prevent duplicates from CI retries.

    # Parameters

    - ``args``: Parsed command-line arguments with benchmarks, machine_types,
      archs, source_dir, target_dir, and max_history.
    """
    max_history: int = getattr(args, "max_history", 0)
    benchmarks = _split_csv_arg(args.benchmarks)
    machines = _split_csv_arg(args.machine_types)
    archs = _split_csv_arg(args.archs)
    groups = list(itertools.product(benchmarks, machines, archs))

    for benchmark, machine, arch in groups:
        filename = gen_filename_for_benchmark(benchmark, machine, arch)
        source_path = os.path.join(args.source_dir, filename)
        target_path = os.path.join(args.target_dir, filename)

        if not os.path.exists(source_path):
            print(f"WARNING: source file not found: {source_path}, skipping.")
            continue

        with open(source_path, "r") as f:
            source_lines = [line.strip() for line in f.readlines() if line.strip()]

        if len(source_lines) < 2:
            print(f"WARNING: source file has no data: {source_path}, skipping.")
            continue

        header = source_lines[0]
        data_rows = source_lines[1:]

        if os.path.exists(target_path):
            # Read the target file once for header validation and
            # duplicate-commit detection.
            with open(target_path, "r") as f:
                existing_content: str = f.read()
            existing_lines: list[str] = [
                line.strip() for line in existing_content.splitlines() if line.strip()
            ]

            existing_header: str = existing_lines[0] if existing_lines else ""
            if existing_header != header:
                if benchmark in SIZE_AWARE_BENCHMARKS:
                    print(
                        f"WARNING: resetting history for {filename}: "
                        f"source='{header}' vs target='{existing_header}'."
                    )
                    new_rows = data_rows
                    with open(target_path, "w") as f:
                        f.write(header + "\n")
                        for row in new_rows:
                            f.write(row + "\n")
                    if max_history > 0:
                        _prune_history(target_path, header, max_history)
                    print(f"Persisted {len(new_rows)} row(s) to {target_path}")
                    continue
                else:
                    print(
                        f"WARNING: header mismatch for {filename}: "
                        f"source='{header}' vs target='{existing_header}'. "
                        f"Skipping."
                    )
                    continue

            # Collect commit hashes already present in the target file to
            # prevent duplicate rows from CI retries.
            existing_commits: set[str] = set()
            for existing_line in existing_lines[1:]:
                parts: list[str] = existing_line.split(",")
                if parts:
                    existing_commits.add(parts[0])

            # Filter out rows whose commit is already persisted.
            new_rows: list[str] = [
                row for row in data_rows if row.split(",")[0] not in existing_commits
            ]

            if not new_rows:
                print(f"No new rows to persist for {filename} (commit already exists).")
                continue

            # Ensure the existing file ends with a newline before appending.
            needs_newline: bool = bool(
                existing_content
            ) and not existing_content.endswith("\n")
            with open(target_path, "a") as f:
                if needs_newline:
                    f.write("\n")
                for row in new_rows:
                    f.write(row + "\n")

            # Prune old rows if max_history is set.
            if max_history > 0:
                _prune_history(target_path, header, max_history)
        else:
            new_rows = data_rows
            with open(target_path, "w") as f:
                f.write(header + "\n")
                for row in new_rows:
                    f.write(row + "\n")

        print(f"Persisted {len(new_rows)} row(s) to {target_path}")


def _prune_history(file_path: str, header: str, max_commits: int) -> None:
    """
    Keep only the last ``max_commits`` unique commits in a history CSV.

    # Parameters

    - ``file_path``: Path to the history CSV file.
    - ``header``: The CSV header line.
    - ``max_commits``: Maximum number of unique commits to retain.
    """
    with open(file_path, "r") as f:
        lines: list[str] = [line.strip() for line in f.readlines() if line.strip()]

    if len(lines) < 2:
        return

    data_lines: list[str] = lines[1:]

    # Collect unique commits in order of appearance.
    seen: set[str] = set()
    ordered_commits: list[str] = []
    for line in data_lines:
        commit: str = line.split(",")[0]
        if commit not in seen:
            seen.add(commit)
            ordered_commits.append(commit)

    if len(ordered_commits) <= max_commits:
        return

    # Keep only rows belonging to the last max_commits commits.
    keep_commits: set[str] = set(ordered_commits[-max_commits:])
    kept_lines: list[str] = [
        row for row in data_lines if row.split(",")[0] in keep_commits
    ]

    with open(file_path, "w") as f:
        f.write(header + "\n")
        for row in kept_lines:
            f.write(row + "\n")

    pruned: int = len(data_lines) - len(kept_lines)
    print(
        f"Pruned {pruned} old row(s) from {file_path} (kept last {max_commits} commits)."
    )


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
    persist_parser = sub_parser.add_parser(
        "persist", help="Append benchmark results to history CSVs"
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
        choices=[MICROVM_MACHINE_TYPE],
        help="Type of machine to run the benchmarks on",
    )
    run_parser.add_argument(
        "--hwloc",
        required=False,
        default=None,
        help="Path to file with the hardware locality information",
    )
    run_parser.add_argument(
        "--bin-dir",
        help="Directory where to find the nanvix-bench binary",
        default="./bin",
    )
    run_parser.add_argument(
        "--iterations",
        type=int,
        default=100,
        help="Path to file with the hardware locality information",
    )
    run_parser.add_argument(
        "--payload-size",
        type=_positive_int,
        default=None,
        help=(
            "Echo payload size in bytes for warm-start-vmm and warm-start-socket benchmarks. "
            "For warm-start-vmm, the size includes "
            "the 4-byte length prefix. Defaults to nanvix-bench's built-in value."
        ),
    )
    run_parser.add_argument(
        "--output-dir",
        help="Directory where to store the benchmark results. If not set prints to stdout",
    )
    run_parser.add_argument(
        "--commit",
        default=None,
        help="Commit SHA to tag this benchmark result. Defaults to HEAD.",
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

    # Command-line arguments for the ci-gate command.
    ci_gate_parser = sub_parser.add_parser(
        "ci-gate",
        help="Check benchmark results for performance regressions against baseline",
    )
    ci_gate_parser.add_argument(
        "--dev-dir", required=True, help="Directory with baseline results from dev"
    )
    ci_gate_parser.add_argument(
        "--target-dir", required=True, help="Directory with current benchmark results"
    )
    ci_gate_parser.add_argument(
        "--benchmarks",
        required=True,
        help="Comma-separated list of benchmarks to check",
    )
    ci_gate_parser.add_argument(
        "--machine-types",
        required=True,
        help="Comma-separated list of machine types",
    )
    ci_gate_parser.add_argument(
        "--archs", required=True, help="Comma-separated list of architectures"
    )
    ci_gate_parser.add_argument(
        "--regression-threshold",
        type=_non_negative_float,
        default=20,
        help="Fail if any benchmark p50 regresses more than this percentage (default: 20)",
    )
    ci_gate_parser.add_argument(
        "--baseline-window",
        type=_positive_int,
        default=20,
        help="Number of most-recent baseline data points to average (default: 20)",
    )
    ci_gate_parser.set_defaults(func=ci_gate)

    # Command-line arguments for the persist command.
    persist_parser.add_argument(
        "--source-dir", required=True, help="Directory with single-run result files"
    )
    persist_parser.add_argument(
        "--target-dir", required=True, help="Target directory with history CSVs"
    )
    persist_parser.add_argument(
        "--benchmarks",
        required=True,
        help="Comma-separated list of benchmarks to persist",
    )
    persist_parser.add_argument(
        "--machine-types",
        required=True,
        help="Comma-separated list of machine types benchmarked",
    )
    persist_parser.add_argument(
        "--archs",
        required=True,
        help="Comma-separated list of architectures benchmarked",
    )
    persist_parser.add_argument(
        "--max-history",
        type=int,
        default=0,
        help="Maximum number of unique commits to keep in history files. 0 means unlimited.",
    )
    persist_parser.set_defaults(func=persist_results)

    args = parser.parse_args()
    # Dispatch the arguments to the selected top-level command.
    result = args.func(args)
    if isinstance(result, int) and result != 0:
        sys.exit(result)
