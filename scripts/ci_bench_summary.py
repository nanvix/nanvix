# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# ======================================================================================================================
# Imports
# ======================================================================================================================

import os
import itertools
import argparse

# ======================================================================================================================
# Standalone Functions
# ======================================================================================================================


def read_metrics(file_path):
    try:
        with open(file_path, "r") as f:
            lines = f.readlines()
            return [int(line.split(" ")[1]) for line in lines]
    except Exception:
        return ["NA", "NA", "NA"]


def make_header(machine, arch, table_width):
    title = f"{machine} ({arch})"
    total_width = table_width
    padding = (
        total_width - len(title) - 2
    ) // 2  # minus 2 for leading and trailing '='
    left = "=" * padding
    right = "=" * (total_width - len(left) - len(title) - 2)
    return f"\n{left} {title} {right}\n"


def generate_tables(dev_dir, target_dir, benchmarks, machines, archs):
    output = "```"
    first_col_width = 6
    col_width = 9
    num_cols_per_bench = 3
    table_width = (
        first_col_width + (col_width * num_cols_per_bench * len(benchmarks)) + 1
    )

    for machine, arch in itertools.product(machines, archs):
        header = make_header(machine, arch, table_width)
        table = [
            [f"{'|':<{first_col_width}}"]
            + [
                f"{b:^{num_cols_per_bench*col_width}}"
                for b in [f"{bench} (us)" for bench in benchmarks]
            ],
            [f"{'|':<{first_col_width}}"]
            + [
                f"{'dev':^{col_width}}",
                f"{'target':^{col_width}}",
                f"{'Δ':^{col_width}}",
            ]
            * len(benchmarks),
            [f"{'| p50':<{first_col_width}}"]
            + [] * len(benchmarks) * num_cols_per_bench,
            [f"{'| p95':<{first_col_width}}"]
            + [] * len(benchmarks) * num_cols_per_bench,
            [f"{'| p99':<{first_col_width}}"]
            + [] * len(benchmarks) * num_cols_per_bench,
        ]

        for benchmark in benchmarks:
            bench_name = benchmark.replace("-", "_")
            filename = f"bench_{bench_name}_{machine}_{arch}.txt"
            dev_path = os.path.join(dev_dir, filename)
            tgt_path = os.path.join(target_dir, filename)

            dev_vals = read_metrics(dev_path)
            tgt_vals = read_metrics(tgt_path)

            for i, val in enumerate(dev_vals):
                table[i + 2].append(f"{val:^{col_width}}")
            for i, val in enumerate(tgt_vals):
                table[i + 2].append(f"{val:^{col_width}}")
            for i, (dev_val, tgt_val) in enumerate(zip(dev_vals, tgt_vals)):
                if tgt_val == "NA" or dev_val == "NA":
                    table[i + 2].append(f"{'NA':^{col_width}}")
                    continue

                pct = float((int(tgt_val) / int(dev_val)) * 100)
                if pct > 100:
                    txt = "+{:.1f}%".format(pct - 100.0)
                else:
                    txt = "-{:.1f}%".format(100.0 - pct)
                table[i + 2].append(f"{txt:^{col_width}}")

        table_str = "\n".join("".join(row) + "|" for row in table)
        footer_str = "\n" + "=" * table_width
        output += header + table_str + footer_str + "\n"

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

    bench_summary = generate_tables(
        args.dev_dir, args.target_dir, benchmarks, machines, archs
    )

    with open(args.output_file, "w") as fh:
        fh.write(bench_summary)
