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
ROUND_TRIP_LATENCY_BENCH = "round-trip-latency"
WARM_START_BENCH = "warm-start"
WARM_START_L2_BENCH = WARM_START_BENCH + L2_SUFFIX
WARM_START_VMM_BENCH = "warm-start-vmm"

# ======================================================================
# Helper Functions
# ======================================================================


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

    bench_summary = "```"

    for benchmark in benchmarks:
        if benchmark in [
            BOOT_TIME_BENCH,
            COLD_START_BENCH,
            COLD_START_L2_BENCH,
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
        else:
            print(f"ERROR: unrecognized benchmark '{benchmark}'")
            raise ValueError("Unrecognized benchmark")

    bench_summary += "```" + "\n"

    with open(args.output_file, "w") as fh:
        fh.write(bench_summary)


def run_benchmark(args):
    """
    Run a single benchmark using nanvix-bench
    """
    print(
        f"Running '{args.benchmark}' benchmark (machine={args.machine_type}, arch={X86_64_ARCH})"
    )

    nanvix_bench_cmd = [
        os.path.join(args.bin_dir, NANVIX_BENCH_ELF),
        f"-benchmark {args.benchmark}",
        f"-hwloc {args.hwloc}",
        f"-iterations {args.iterations}",
        f"-tmp-dir {args.tmp_dir}" if args.tmp_dir is not None else "",
        f"-toolchain-bin-dir {args.toolchain_bin_dir}",
    ]
    nanvix_bench_cmd = " ".join(nanvix_bench_cmd)

    if args.output_dir is not None:
        output_dir = pathlib.Path(args.output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)
        output_file = os.path.join(
            output_dir,
            gen_filename_for_benchmark(args.benchmark, args.machine_type, X86_64_ARCH),
        )

        # Run benchmark and capture raw stdout.
        raw_stdout = subprocess.run(
            nanvix_bench_cmd, shell=True, capture_output=True
        ).stdout.decode("utf-8")
        filtered_stdout = filter_benchmark_stdout(args.benchmark, raw_stdout)
        with open(output_file, "w") as fh:
            fh.write(filtered_stdout)
    else:
        subprocess.run(nanvix_bench_cmd, shell=True, check=True)


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
        "--tmp-dir",
        help="Temporary directory to run the benchmarks in",
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
