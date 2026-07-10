# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
# Description

Generate performance plots for Nanvix benchmarks.

The script auto-discovers ``nanvix_bench_*.csv`` files under the data
directory's four subdirectories:

- ``baremetal/`` — Linux self-hosted runner results
- ``github/`` — Linux GitHub runner results
- ``windows-baremetal/`` — Windows self-hosted runner results
- ``windows-github/`` — Windows GitHub runner results

It keeps only the last *N* commits (default 100) and produces PNG plots for
each benchmark: single-series benchmarks generate one PNG per benchmark, while
sized benchmarks generate one PNG per message size.  Platform- and
runner-specific plots are saved with ``linux_baremetal_``, ``linux_github_``,
``windows_baremetal_``, and ``windows_github_`` prefixes.

Benchmarks whose CSV files follow the ``commit,p50,p95,p99`` schema and the
``commit,size,p50,p95,p99`` schema (e.g. ``round_trip_latency``) are
supported.  Benchmarks with other schemas (e.g. ``echo_breakdown`` which
uses ``commit,step,label,p50,p95,p99``) are not yet supported and will be
skipped.

# Usage

::

    python scripts/plot-performance.py                 # defaults
    python scripts/plot-performance.py --max-commits 50 --output-dir plots
    python scripts/plot-performance.py --data-dir data --benchmarks cold_start warm_start
"""

# ======================================================================
# Imports
# ======================================================================

import argparse
import csv
import dataclasses
import os
import pathlib
import subprocess
import sys
from typing import Optional

try:
    import matplotlib
    import matplotlib.pyplot as plt
    import matplotlib.ticker as ticker
except ImportError:
    print(
        "ERROR: matplotlib is not installed. "
        "Run 'make python-init' to set up the Python virtual environment."
    )
    sys.exit(1)

# Use a non-interactive backend so the script works in headless CI
# environments without a display server.
matplotlib.use("Agg")

# ======================================================================
# Constants
# ======================================================================

MAX_COMMITS: int = 100
DATA_DIR: str = "data"
BAREMETAL_SUBDIR: str = "baremetal"
GITHUB_SUBDIR: str = "github"
WINDOWS_BAREMETAL_SUBDIR: str = "windows-baremetal"
WINDOWS_GITHUB_SUBDIR: str = "windows-github"
OUTPUT_DIR: str = "plots"
SHORT_SHA_LEN: int = 7

# Supported benchmarks whose CSV files follow the ``commit,p50,p95,p99``
# schema.  Benchmarks with different schemas are intentionally excluded:
#   - ``echo_breakdown``: uses ``commit,step,label,p50,p95,p99``.
SUPPORTED_BENCHMARKS: list[str] = [
    "boot_time",
    "cold_start",
    "cold_start_uvm",
    "snapshot_restore",
]

# Benchmarks whose CSV files follow the ``commit,size,p50,p95,p99`` schema.
# Each produces one plot per message size.
SIZED_BENCHMARKS: list[str] = [
    "round_trip_latency",
    "warm_start",
    "warm_start_socket",
    "warm_start_vmm",
]

# Human-readable titles for each benchmark.
BENCHMARK_TITLES: dict[str, str] = {
    "boot_time": "Boot Time",
    "cold_start": "Cold Start",
    "cold_start_uvm": "Cold Start (UVM)",
    "echo_breakdown": "Echo Breakdown",
    "round_trip_latency": "Round-Trip Latency",
    "snapshot_restore": "Snapshot Restore",
    "warm_start": "Warm Start",
    "warm_start_socket": "Warm Start Socket",
    "warm_start_vmm": "Warm Start (VMM)",
}

# Y-axis label for all latency plots.
Y_LABEL: str = "Latency (μs)"

# Percentile columns in CSV order.
PERCENTILE_LABELS: list[str] = ["p50", "p95", "p99"]

# Visual styles cycled per machine in multi-machine plots.
MACHINE_STYLES: list[dict[str, str]] = [
    {"marker": "o", "color": "tab:blue"},
    {"marker": "s", "color": "tab:orange"},
    {"marker": "^", "color": "tab:green"},
    {"marker": "D", "color": "tab:red"},
]


# ======================================================================
# Data Classes
# ======================================================================


@dataclasses.dataclass
class CsvFileInfo:
    """Metadata for a discovered benchmark CSV file.

    # Description

    Holds the parsed benchmark name, target machine, architecture, and
    the absolute path to the CSV file.
    """

    bench_name: str
    machine: str
    arch: str
    file_path: str


# ======================================================================
# Helper Functions
# ======================================================================


def short_sha(commit: str) -> str:
    """Return the first ``SHORT_SHA_LEN`` characters of a commit SHA.

    # Parameters

    - ``commit``: Full commit SHA string.

    # Returns

    Shortened commit SHA.
    """
    return commit[:SHORT_SHA_LEN]


def discover_csv_files(data_dir: str) -> list[CsvFileInfo]:
    """Discover benchmark CSV files in *data_dir*.

    Files must match the naming convention
    ``nanvix_bench_<benchmark>_<machine>_<arch>.csv``.

    # Parameters

    - ``data_dir``: Directory to scan for CSV files.

    # Returns

    A list of ``CsvFileInfo`` objects, one per discovered CSV file.
    """
    result: list[CsvFileInfo] = []
    prefix: str = "nanvix_bench_"
    data_path: pathlib.Path = pathlib.Path(data_dir)

    if not data_path.is_dir():
        print(f"ERROR: data directory '{data_dir}' does not exist.")
        sys.exit(1)

    for csv_file in sorted(data_path.glob(f"{prefix}*.csv")):
        # Strip the prefix and the last two underscore-separated tokens
        # (machine type and architecture) to recover the benchmark name.
        stem: str = csv_file.stem  # e.g. nanvix_bench_boot_time_microvm_X64
        tokens: list[str] = stem[len(prefix) :].rsplit("_", 2)
        if len(tokens) < 3:
            continue
        result.append(
            CsvFileInfo(
                bench_name=tokens[0],
                machine=tokens[1],
                arch=tokens[2],
                file_path=str(csv_file.resolve()),
            )
        )

    return result


def read_csv(file_path: str) -> tuple[list[str], list[list[str]]]:
    """Read a CSV file and return its header and data rows.

    # Parameters

    - ``file_path``: Path to the CSV file.

    # Returns

    A tuple of (header columns, data rows).
    """
    with open(file_path, "r", newline="") as f:
        reader = csv.reader(f)
        header: list[str] = next(reader)
        rows: list[list[str]] = [row for row in reader if row]
    return header, rows


def unique_commits_ordered(rows: list[list[str]]) -> list[str]:
    """Return unique commit SHAs from data rows, preserving order.

    # Parameters

    - ``rows``: Data rows where the first column is the commit SHA.

    # Returns

    List of unique commit SHAs in order of first appearance.
    """
    seen: set[str] = set()
    ordered: list[str] = []
    for row in rows:
        commit: str = row[0]
        if commit not in seen:
            seen.add(commit)
            ordered.append(commit)
    return ordered


def git_commit_order(commits: set[str]) -> list[str]:
    """Sort *commits* in chronological order using ``git log``.

    Runs ``git log --format=%H --reverse`` to obtain the authoritative
    commit ordering from the repository history.  Commits not found in
    the log (e.g. from other branches) are appended at the end.

    # Parameters

    - ``commits``: Set of full commit SHAs to sort.

    # Returns

    A list of the supplied SHAs in chronological (oldest-first) order.
    """
    try:
        result: subprocess.CompletedProcess[str] = subprocess.run(
            ["git", "log", "--format=%H", "--reverse"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (subprocess.CalledProcessError, FileNotFoundError):
        # Fallback: return commits in arbitrary order when git is
        # unavailable (e.g. in CI without a checkout).
        return sorted(commits)

    ordered: list[str] = [sha for sha in result.stdout.splitlines() if sha in commits]
    # Append any commits that were not found in the log.
    remaining: set[str] = commits - set(ordered)
    ordered.extend(sorted(remaining))
    return ordered


def keep_last_n_commits(
    rows: list[list[str]], max_commits: int
) -> tuple[list[str], list[list[str]]]:
    """Trim rows to only retain the last *max_commits* unique commits.

    # Parameters

    - ``rows``: All data rows.
    - ``max_commits``: Maximum number of commits to keep.

    # Returns

    A tuple of (kept commits in order, filtered rows).
    """
    all_commits: list[str] = unique_commits_ordered(rows)
    if len(all_commits) > max_commits:
        all_commits = all_commits[-max_commits:]
    commit_set: set[str] = set(all_commits)
    filtered: list[list[str]] = [r for r in rows if r[0] in commit_set]
    return all_commits, filtered


def _configure_axes(
    ax: matplotlib.axes.Axes,
    commits: list[str],
    title: str,
    ylabel: str,
) -> None:
    """Apply common visual settings to an axes object.

    # Parameters

    - ``ax``: The matplotlib axes to configure.
    - ``commits``: Ordered list of commit SHAs (used for x-tick labels).
    - ``title``: Plot title.
    - ``ylabel``: Y-axis label.
    """
    ax.set_title(title, fontsize=14, fontweight="bold")
    ax.set_ylabel(ylabel)
    ax.set_xlabel("Commit")
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda v, _: f"{int(v):,}"))
    ax.grid(axis="y", linestyle="--", alpha=0.5)
    ax.legend(fontsize=8, loc="best")

    # Show x-tick labels only when the number of commits is manageable.
    if len(commits) <= 30:
        ax.set_xticks(range(len(commits)))
        ax.set_xticklabels(
            [short_sha(c) for c in commits], rotation=60, ha="right", fontsize=7
        )
    else:
        # For many commits, show a subset of ticks to avoid overlap.
        step: int = max(1, len(commits) // 20)
        positions: list[int] = list(range(0, len(commits), step))
        ax.set_xticks(positions)
        ax.set_xticklabels(
            [short_sha(commits[i]) for i in positions],
            rotation=60,
            ha="right",
            fontsize=7,
        )


# ======================================================================
# Plotting Functions
# ======================================================================


def plot_benchmark_multiplot(
    bench_name: str,
    csv_infos: list[CsvFileInfo],
    output_dir: str,
    max_commits: int,
    file_prefix: str = "",
) -> Optional[str]:
    """Plot a benchmark with one subplot per percentile and one line per machine.

    Produces a figure with three vertically-stacked subplots (p50, p95, p99).
    Each subplot contains one line per machine (e.g. Microvm).

    # Parameters

    - ``bench_name``: Short benchmark identifier.
    - ``csv_infos``: List of ``CsvFileInfo`` objects for different machines.
    - ``output_dir``: Directory where the PNG will be saved.
    - ``max_commits``: Maximum number of commits to plot.
    - ``file_prefix``: Optional prefix for the output filename.

    # Returns

    Path to the generated PNG file, or ``None`` on failure.
    """
    # Read data for each machine and merge all rows to build a unified
    # commit timeline.
    machine_rows: dict[str, list[list[str]]] = {}
    all_commit_shas: set[str] = set()
    for info in csv_infos:
        _, rows = read_csv(info.file_path)
        if rows:
            valid_rows: list[list[str]] = []
            for row in rows:
                if len(row) < 4:
                    print(
                        f"WARNING: skipping malformed row in '{info.file_path}': {row}"
                    )
                    continue
                valid_rows.append(row)
                all_commit_shas.add(row[0])
            if valid_rows:
                machine_rows[info.machine] = valid_rows

    if not machine_rows:
        print(f"WARNING: no data for '{bench_name}', skipping.")
        return None

    # Sort commits chronologically using the git history.
    commits: list[str] = git_commit_order(all_commit_shas)
    if len(commits) > max_commits:
        commits = commits[-max_commits:]
    commit_set: set[str] = set(commits)
    commit_idx: dict[str, int] = {c: i for i, c in enumerate(commits)}

    # For each machine build a mapping: commit -> (p50, p95, p99).
    machine_values: dict[str, dict[str, tuple[int, int, int]]] = {}
    for machine, rows in machine_rows.items():
        commit_map: dict[str, tuple[int, int, int]] = {}
        for row in rows:
            if row[0] in commit_set:
                commit_map[row[0]] = (int(row[1]), int(row[2]), int(row[3]))
        machine_values[machine] = commit_map

    # Create one subplot per percentile.
    num_percentiles: int = len(PERCENTILE_LABELS)
    fig, axes = plt.subplots(
        num_percentiles,
        1,
        figsize=(max(10, len(commits) * 0.35), 5 * num_percentiles),
        sharex=True,
    )

    sorted_machines: list[str] = sorted(machine_values.keys())
    for p_idx, (ax, p_label) in enumerate(zip(axes, PERCENTILE_LABELS)):
        for m_idx, machine in enumerate(sorted_machines):
            commit_map = machine_values[machine]
            x: list[int] = []
            y: list[int] = []
            for commit in commits:
                if commit in commit_map:
                    x.append(commit_idx[commit])
                    y.append(commit_map[commit][p_idx])
            style: dict[str, str] = MACHINE_STYLES[m_idx % len(MACHINE_STYLES)]
            ax.plot(
                x,
                y,
                marker=style["marker"],
                color=style["color"],
                markersize=3,
                linewidth=1.2,
                label=machine.title(),
            )

        title: str = f"{BENCHMARK_TITLES.get(bench_name, bench_name)} \u2014 {p_label}"
        _configure_axes(ax, commits, title, Y_LABEL)

    fig.tight_layout()
    out_name: str = f"{file_prefix}{bench_name}"
    out_path: str = os.path.join(output_dir, f"{out_name}.png")
    fig.savefig(out_path, dpi=150)
    plt.close(fig)
    return out_path


def plot_sized_benchmark_multiplot(
    bench_name: str,
    csv_infos: list[CsvFileInfo],
    output_dir: str,
    max_commits: int,
    file_prefix: str = "",
) -> list[str]:
    """Plot a sized benchmark with one PNG per message size.

    Each PNG contains three vertically-stacked subplots (p50, p95, p99)
    with one line per machine, identical in layout to the standard
    multiplot but filtered to a single message size.

    CSV schema: ``commit,size,p50,p95,p99``.

    # Parameters

    - ``bench_name``: Short benchmark identifier.
    - ``csv_infos``: List of ``CsvFileInfo`` objects for different machines.
    - ``output_dir``: Directory where the PNGs will be saved.
    - ``max_commits``: Maximum number of commits to plot.
    - ``file_prefix``: Optional prefix for the output filename.

    # Returns

    List of paths to the generated PNG files.
    """
    # Read data for each machine, keyed by (machine, size, commit).
    machine_rows: dict[str, list[list[str]]] = {}
    all_commit_shas: set[str] = set()
    all_sizes_ordered: list[str] = []
    seen_sizes: set[str] = set()

    for info in csv_infos:
        _, rows = read_csv(info.file_path)
        if rows:
            valid_rows: list[list[str]] = []
            for row in rows:
                if len(row) < 5:
                    print(
                        f"WARNING: skipping malformed row in '{info.file_path}': {row}"
                    )
                    continue
                valid_rows.append(row)
                all_commit_shas.add(row[0])
                size_val: str = row[1]
                if size_val not in seen_sizes:
                    seen_sizes.add(size_val)
                    all_sizes_ordered.append(size_val)
            if valid_rows:
                machine_rows[info.machine] = valid_rows

    if not machine_rows:
        print(f"WARNING: no data for '{bench_name}', skipping.")
        return []

    # Sort commits chronologically using the git history.
    commits: list[str] = git_commit_order(all_commit_shas)
    if len(commits) > max_commits:
        commits = commits[-max_commits:]
    commit_set: set[str] = set(commits)
    commit_idx: dict[str, int] = {c: i for i, c in enumerate(commits)}

    # For each machine build a mapping: (size, commit) -> (p50, p95, p99).
    machine_values: dict[str, dict[tuple[str, str], tuple[int, int, int]]] = {}
    active_sizes: set[str] = set()
    for machine, rows in machine_rows.items():
        vals: dict[tuple[str, str], tuple[int, int, int]] = {}
        for row in rows:
            if row[0] in commit_set:
                vals[(row[1], row[0])] = (int(row[2]), int(row[3]), int(row[4]))
                active_sizes.add(row[1])
        machine_values[machine] = vals

    # Keep only sizes that have data in the selected commit window.
    all_sizes_ordered = [s for s in all_sizes_ordered if s in active_sizes]

    generated: list[str] = []
    sorted_machines: list[str] = sorted(machine_values.keys())
    bench_title: str = BENCHMARK_TITLES.get(bench_name, bench_name)
    num_percentiles: int = len(PERCENTILE_LABELS)

    for size in all_sizes_ordered:
        fig, axes = plt.subplots(
            num_percentiles,
            1,
            figsize=(max(10, len(commits) * 0.35), 5 * num_percentiles),
            sharex=True,
        )

        for p_idx, (ax, p_label) in enumerate(zip(axes, PERCENTILE_LABELS)):
            for m_idx, machine in enumerate(sorted_machines):
                vals = machine_values[machine]
                x: list[int] = []
                y: list[int] = []
                for commit in commits:
                    key = (size, commit)
                    if key in vals:
                        x.append(commit_idx[commit])
                        y.append(vals[key][p_idx])
                style: dict[str, str] = MACHINE_STYLES[m_idx % len(MACHINE_STYLES)]
                ax.plot(
                    x,
                    y,
                    marker=style["marker"],
                    color=style["color"],
                    markersize=3,
                    linewidth=1.2,
                    label=machine.title(),
                )

            title: str = f"{bench_title} ({size}) \u2014 {p_label}"
            _configure_axes(ax, commits, title, Y_LABEL)

        fig.tight_layout()
        # Sanitize size token for filename (e.g. "1KiB" -> "1KiB").
        size_token: str = size.replace("/", "_")
        out_name: str = f"{file_prefix}{bench_name}_{size_token}"
        out_path: str = os.path.join(output_dir, f"{out_name}.png")
        fig.savefig(out_path, dpi=150)
        plt.close(fig)
        generated.append(out_path)

    return generated


# ======================================================================
# Main
# ======================================================================


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments.

    # Returns

    Parsed argument namespace.
    """
    parser: argparse.ArgumentParser = argparse.ArgumentParser(
        description="Generate performance plots from Nanvix benchmark CSVs.",
    )
    parser.add_argument(
        "--data-dir",
        type=str,
        default=DATA_DIR,
        help=f"Directory containing benchmark CSV files (default: {DATA_DIR}).",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default=OUTPUT_DIR,
        help=f"Directory where PNG plots are saved (default: {OUTPUT_DIR}).",
    )
    parser.add_argument(
        "--max-commits",
        type=int,
        default=MAX_COMMITS,
        help=f"Maximum number of recent commits to plot (default: {MAX_COMMITS}).",
    )
    parser.add_argument(
        "--benchmarks",
        nargs="*",
        default=None,
        help="Restrict plotting to these benchmark names (default: all cold/warm start).",
    )
    return parser.parse_args()


def _plot_csv_files(
    csv_files: list[CsvFileInfo],
    output_dir: str,
    max_commits: int,
    file_prefix: str = "",
) -> list[str]:
    """Generate multiplots for discovered CSV files, grouped by benchmark.

    For standard benchmarks a single PNG is produced with three subplots
    (p50, p95, p99) showing one line per machine type.  Sized benchmarks
    produce one PNG per message size with the same subplot layout.

    # Parameters

    - ``csv_files``: List of discovered CSV file metadata.
    - ``output_dir``: Directory where PNG plots are saved.
    - ``max_commits``: Maximum number of recent commits to plot.
    - ``file_prefix``: Optional prefix for output filenames (e.g. ``github_``).

    # Returns

    List of paths to generated PNG files.
    """
    # Group CSV files by benchmark name.
    bench_groups: dict[str, list[CsvFileInfo]] = {}
    sized_groups: dict[str, list[CsvFileInfo]] = {}
    for info in sorted(csv_files, key=lambda i: i.file_path):
        if info.bench_name in SIZED_BENCHMARKS:
            sized_groups.setdefault(info.bench_name, []).append(info)
        elif info.bench_name in SUPPORTED_BENCHMARKS:
            bench_groups.setdefault(info.bench_name, []).append(info)
        else:
            print(f"Skipping unsupported benchmark '{info.bench_name}'.")
            continue

    generated: list[str] = []
    for bench_name in sorted(bench_groups.keys()):
        infos: list[CsvFileInfo] = bench_groups[bench_name]
        machines: str = ", ".join(i.machine for i in infos)
        print(f"Plotting {file_prefix}{bench_name} ({machines}) ...")

        out: Optional[str] = plot_benchmark_multiplot(
            bench_name, infos, output_dir, max_commits, file_prefix=file_prefix
        )
        if out:
            generated.append(out)

    for bench_name in sorted(sized_groups.keys()):
        infos = sized_groups[bench_name]
        machines = ", ".join(i.machine for i in infos)
        print(f"Plotting {file_prefix}{bench_name} ({machines}) ...")

        outs: list[str] = plot_sized_benchmark_multiplot(
            bench_name, infos, output_dir, max_commits, file_prefix=file_prefix
        )
        generated.extend(outs)

    return generated


def _discover_subdir(data_dir: str, subdir: str) -> list[CsvFileInfo]:
    """Discover benchmark CSV files in a subdirectory of *data_dir*.

    # Parameters

    - ``data_dir``: Root data directory.
    - ``subdir``: Subdirectory name to scan.

    # Returns

    A list of ``CsvFileInfo`` objects, or an empty list if the
    subdirectory does not exist.
    """
    path: str = os.path.join(data_dir, subdir)
    if os.path.isdir(path):
        return discover_csv_files(path)
    return []


def main() -> None:
    """Entry point: discover CSVs, generate plots, and report results."""
    args: argparse.Namespace = parse_args()

    # Discover benchmark data from all platform/runner subdirectories.
    sources: list[tuple[str, str, list[CsvFileInfo]]] = [
        (
            "Linux bare-metal",
            "linux_baremetal_",
            _discover_subdir(args.data_dir, BAREMETAL_SUBDIR),
        ),
        (
            "Linux GitHub runner",
            "linux_github_",
            _discover_subdir(args.data_dir, GITHUB_SUBDIR),
        ),
        (
            "Windows bare-metal",
            "windows_baremetal_",
            _discover_subdir(args.data_dir, WINDOWS_BAREMETAL_SUBDIR),
        ),
        (
            "Windows GitHub runner",
            "windows_github_",
            _discover_subdir(args.data_dir, WINDOWS_GITHUB_SUBDIR),
        ),
    ]

    # If no known subdirectories were found, try data_dir itself as a
    # flat directory containing CSV files (e.g. when the user points
    # directly at a single subdirectory).
    if not any(csv_files for _, _, csv_files in sources):
        flat_files: list[CsvFileInfo] = discover_csv_files(args.data_dir)
        if flat_files:
            sources = [("custom", "", flat_files)]

    if not any(csv_files for _, _, csv_files in sources):
        print(f"ERROR: no CSV files found in '{args.data_dir}'.")
        sys.exit(1)

    # If the user requested specific benchmarks, filter the discovered files.
    all_supported: set[str] = set(SUPPORTED_BENCHMARKS) | set(SIZED_BENCHMARKS)
    if args.benchmarks:
        all_known: set[str] = set()
        for _, _, csv_files in sources:
            all_known |= {i.bench_name for i in csv_files}
        unknown: list[str] = [b for b in args.benchmarks if b not in all_known]
        if unknown:
            print(f"WARNING: unknown benchmarks ignored: {unknown}")
        unsupported: list[str] = [
            b for b in args.benchmarks if b in all_known and b not in all_supported
        ]
        if unsupported:
            print(f"WARNING: unsupported benchmarks skipped: {unsupported}")
        bench_set: set[str] = set(args.benchmarks)
        sources = [
            (label, prefix, [i for i in csv_files if i.bench_name in bench_set])
            for label, prefix, csv_files in sources
        ]

    os.makedirs(args.output_dir, exist_ok=True)

    generated: list[str] = []

    for label, prefix, csv_files in sources:
        if csv_files:
            print(f"--- {label} benchmarks ---")
            generated.extend(
                _plot_csv_files(
                    csv_files,
                    args.output_dir,
                    args.max_commits,
                    file_prefix=prefix,
                )
            )

    if generated:
        print(f"Generated {len(generated)} plot(s) in '{args.output_dir}/':")
        for path in generated:
            print(f"  - {path}")
    else:
        print("WARNING: no plots were generated.")


if __name__ == "__main__":
    main()
