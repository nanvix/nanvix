# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# ======================================================================
# Imports
# ======================================================================

import os
import itertools
import argparse

# ======================================================================
# Constants
# ======================================================================

PERCENTILES = ["p50", "p95", "p99"]
ROUND_TRIP_LATENCY_BENCH = "round-trip-latency"

# ======================================================================
# Standalone Functions
# ======================================================================


# Helper function to decide if the benchmark is part of the round-trip-latency
# benchmarks or not. We have multiple tables for this benchmark to capture the
# different percentiles for all message sizes.
def is_round_trip_latency_bench(benchmark):
    return benchmark.startswith(ROUND_TRIP_LATENCY_BENCH)


def read_metrics(benchmark, file_path):
    if not is_round_trip_latency_bench(benchmark):
        try:
            with open(file_path, "r") as f:
                lines = f.readlines()
                return [int(line.split(" ")[1]) for line in lines]
        except Exception:
            return ["NA", "NA", "NA"]
    else:
        percentile = benchmark[-3:]
        # The benchmark results are formatted like:
        # size:\tp50\tp95\tp90
        # so the index for each percentile is 1 + their index in PERCENTILES.
        line_idx = PERCENTILES.index(percentile) + 1
        try:
            print(file_path)
            with open(file_path, "r") as f:
                lines = f.readlines()
                lines = [line.strip() for line in lines]
                print(lines[0])
                print(lines[0].split("\t"))
                return [int(line.split("\t")[line_idx]) for line in lines]
        except Exception:
            return ["NA", "NA", "NA", "NA", "NA", "NA"]


def make_header(benchmark, table_width):
    title = f"{benchmark} (us)"
    total_width = table_width
    padding = (
        total_width - len(title) - 2
    ) // 2  # minus 2 for leading and trailing '='
    left = "=" * padding
    right = "=" * (total_width - len(left) - len(title) - 2)
    return f"\n{left} {title} {right}\n"


def generate_tables(dev_dir, target_dir, benchmarks, machines, archs):
    output = "```"

    for benchmark in benchmarks:
        # Calculate table dimensions
        first_col_width = 7  # "| p50 "
        sub_col_width = 12  # Width for each sub-column (dev, target, delta)
        machine_col_width = sub_col_width * 3  # 3 sub-columns per machine

        # Number of column groups is cartesian product of machines and archs
        groups = list(itertools.product(machines, archs))
        groups_count = len(groups)

        # Calculate total table width correctly
        # 2 for leading and trailing '|', plus first column width, plus for each group
        # 3 sub-columns (machine_col_width) and 3 internal separators between them.
        # This yields the exact length of any data/sub-header row.
        table_width = 2 + first_col_width + (groups_count * (machine_col_width + 3))

        # Create header for this benchmark
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

        print("tmp val", is_round_trip_latency_bench(benchmark))
        if not is_round_trip_latency_bench(benchmark):
            # Data rows (p50, p95, p99)
            print(f"FOO: {benchmark}")
            rows = PERCENTILES
        else:
            # We should consider parsing these values from the results file.
            rows = ["32 B", "64 B", "128 B", "256 B", "512 B", "1 KiB", "4 KiB"]

        for row_idx, row_label in enumerate(rows):
            row_parts = [f" {row_label:^{first_col_width-2}} "]

            for machine, arch in groups:
                bench_name = benchmark.replace("-", "_")
                if is_round_trip_latency_bench(benchmark):
                    bench_name = bench_name[:-4]

                filename = f"bench_{bench_name}_{machine}_{arch}.txt"
                dev_path = os.path.join(dev_dir, filename)
                tgt_path = os.path.join(target_dir, filename)

                dev_vals = read_metrics(benchmark, dev_path)
                tgt_vals = read_metrics(benchmark, tgt_path)

                dev_val = dev_vals[row_idx] if row_idx < len(dev_vals) else "NA"
                tgt_val = tgt_vals[row_idx] if row_idx < len(tgt_vals) else "NA"

                # Calculate delta
                if tgt_val == "NA" or dev_val == "NA":
                    delta_str = "NA"
                else:
                    pct = float((int(tgt_val) / int(dev_val)) * 100)
                    if pct > 100:
                        delta_str = "+{:.1f}%".format(pct - 100.0)
                    else:
                        delta_str = "-{:.1f}%".format(100.0 - pct)

                # Add each sub-column separately
                row_parts.extend(
                    [
                        f"{dev_val:^{sub_col_width}}",
                        f"{tgt_val:^{sub_col_width}}",
                        f"{delta_str:^{sub_col_width}}",
                    ]
                )

            # Join the row parts with | separators
            row_line = "|" + "|".join(row_parts) + "|"
            table_lines.append(row_line)

        # Add footer
        footer_str = "=" * table_width

        # Combine everything for this benchmark
        table_str = "\n".join(table_lines)
        output += header + table_str + "\n" + footer_str + "\n"

    output += "```" + "\n"
    return output


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generate a summary of the micro-benchmark results"
    )
    parser.add_argument(
        "--dev-dir", required=True, help="Directory for the results from the dev branch"
    )
    parser.add_argument(
        "--target-dir",
        required=True,
        help="Directory for the results from the current branch",
    )
    parser.add_argument(
        "--benchmarks",
        required=True,
        help="Comma-separated list of benchmarks to aggregate",
    )
    parser.add_argument(
        "--machine-types",
        required=True,
        help="Comma-separated list of machines types benchmarked",
    )
    parser.add_argument(
        "--archs",
        required=True,
        help="Comma-separated list of architectures benchmarked",
    )
    parser.add_argument(
        "--output-file", required=True, help="File to output the benchmark summary"
    )
    args = parser.parse_args()

    benchmarks = args.benchmarks.split(",")
    machines = args.machine_types.split(",")
    archs = args.archs.split(",")

    # For the round-trip-latency benchmark we generate three tables, one for
    # p50, one for p95, and one for p99.
    if ROUND_TRIP_LATENCY_BENCH in benchmarks:
        benchmarks.remove(ROUND_TRIP_LATENCY_BENCH)
        for percentile in PERCENTILES:
            benchmarks.append(f"{ROUND_TRIP_LATENCY_BENCH}-{percentile}")

    bench_summary = generate_tables(
        args.dev_dir, args.target_dir, benchmarks, machines, archs
    )

    with open(args.output_file, "w") as fh:
        fh.write(bench_summary)
