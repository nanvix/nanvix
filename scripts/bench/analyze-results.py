#!/usr/bin/env python3
"""
Nanvix Benchmark Analysis Tool
===============================

Parses nanvix-bench output (stdout text tables + stderr PERF_TIMINGS JSON)
and generates analysis reports with bottleneck insights.

Usage:
    # Analyze a benchmark run
    python analyze-results.py report --stdout results-stdout.txt --stderr results-stderr.txt

    # Compare two runs (A/B testing)
    python analyze-results.py compare --before baseline-stderr.txt --after optimized-stderr.txt

    # Compare against a baseline CSV
    python analyze-results.py report --stdout results-stdout.txt --stderr results-stderr.txt \\
        --baseline baseline.csv

    # Output JSON report
    python analyze-results.py report --stdout results-stdout.txt --stderr results-stderr.txt \\
        --json report.json

Dependencies: Python 3.10+ (stdlib only)
"""

import argparse
import json
import math
import os
import re
import sys
from collections import OrderedDict

# ── Parsing ──────────────────────────────────────────────────────────────────


def parse_bench_stdout(text):
    """Parse nanvix-bench text output into structured data.

    Expected format:
        First req: 88859 us
        p50: 49618 us
        p95: 54033 us
        p99: 88859 us

        Phase                    p50 (us)   p95 (us)   p99 (us)
        ------------------------------------------------------
        channel_setup                  10         18         21
        ...
        total                       59206      61852      62578
    """
    result = {"summary": {}, "phases": OrderedDict()}

    # Parse summary lines (First req, p50, p95, p99)
    for match in re.finditer(r"(First req|p50|p95|p99):\s*(\d+)\s*us", text):
        key = match.group(1).replace(" ", "_").lower()
        result["summary"][key] = int(match.group(2))

    # Parse phase table
    phase_pattern = re.compile(
        r"^(\w+)\s+"  # phase name
        r"(\d+)\s+"  # p50
        r"(\d+)\s+"  # p95
        r"(\d+)\s*$",  # p99
        re.MULTILINE,
    )
    for match in phase_pattern.finditer(text):
        name = match.group(1)
        result["phases"][name] = {
            "p50": int(match.group(2)),
            "p95": int(match.group(3)),
            "p99": int(match.group(4)),
        }

    return result


def parse_perf_timings(text):
    """Parse PERF_TIMINGS:{json} lines from stderr.

    Each line is one iteration's per-phase timing in microseconds.
    Returns list of dicts, one per iteration.
    """
    iterations = []
    for line in text.split("\n"):
        line = line.strip()
        if not line.startswith("PERF_TIMINGS:"):
            continue
        json_str = line[len("PERF_TIMINGS:") :]
        try:
            data = json.loads(json_str)
            iterations.append(data)
        except json.JSONDecodeError:
            continue
    return iterations


def parse_baseline_csv(path):
    """Parse a baseline CSV into a dict.

    Supported formats:
        - commit,p50,p95,p99
        - phase,p50,p95,p99

    Returns dict with keys = phase names, values = {p50, p95, p99}.
    For commit-level baselines, the parsed values are stored under "total".
    """
    result = {}
    with open(path, "r") as f:
        header = f.readline().strip().split(",")
        if len(header) < 2:
            return result

        # Standard format: commit,p50,p95,p99
        if header == ["commit", "p50", "p95", "p99"]:
            for line in f:
                parts = line.strip().split(",")
                if len(parts) >= 4:
                    result["total"] = {
                        "p50": int(parts[1]),
                        "p95": int(parts[2]),
                        "p99": int(parts[3]),
                    }

        # Phase format: phase,p50,p95,p99
        elif header[0] == "phase":
            for line in f:
                parts = line.strip().split(",")
                if len(parts) >= 4:
                    result[parts[0]] = {
                        "p50": int(parts[1]),
                        "p95": int(parts[2]),
                        "p99": int(parts[3]),
                    }

    return result


# ── Statistics ───────────────────────────────────────────────────────────────


def compute_stats(values):
    """Compute descriptive statistics for a list of numeric values."""
    if not values:
        return {}

    n = len(values)
    sorted_vals = sorted(values)
    mean = sum(values) / n
    variance = sum((x - mean) ** 2 for x in values) / (n - 1) if n > 1 else 0
    stddev = math.sqrt(variance)
    cv = (stddev / mean * 100) if mean > 0 else 0

    return {
        "count": n,
        "mean": round(mean, 1),
        "stddev": round(stddev, 1),
        "min": sorted_vals[0],
        "max": sorted_vals[-1],
        "p50": sorted_vals[min(math.ceil(n * 0.50) - 1, n - 1)],
        "p95": sorted_vals[min(math.ceil(n * 0.95) - 1, n - 1)],
        "p99": sorted_vals[min(math.ceil(n * 0.99) - 1, n - 1)],
        "cv_pct": round(cv, 1),
    }


def compute_phase_distribution(iterations):
    """Compute per-phase statistics from PERF_TIMINGS iterations."""
    if not iterations:
        return {}

    # Collect all phase names
    all_phases = OrderedDict()
    for it in iterations:
        for phase in it:
            if phase not in all_phases:
                all_phases[phase] = []

    # Collect values
    for it in iterations:
        for phase in all_phases:
            val = it.get(phase)
            if val is not None:
                all_phases[phase].append(val)

    # Compute stats
    return {phase: compute_stats(vals) for phase, vals in all_phases.items()}


# ── Regression Detection ─────────────────────────────────────────────────────


def detect_regressions(current_phases, baseline_phases, warn_pct=5, alert_pct=10):
    """Compare current phase p50 values against baseline.

    Returns list of findings with severity:
        improvement: >warn_pct faster
        warning:     >warn_pct slower
        alert:       >alert_pct slower
    """
    findings = []
    for phase, current in current_phases.items():
        if phase not in baseline_phases:
            continue
        baseline = baseline_phases[phase]

        curr_val = current.get("p50", 0)
        base_val = baseline.get("p50", 0)
        if base_val == 0:
            continue

        delta_pct = (curr_val - base_val) / base_val * 100

        if delta_pct > alert_pct:
            findings.append(
                {
                    "phase": phase,
                    "severity": "alert",
                    "delta_pct": round(delta_pct, 1),
                    "current_us": curr_val,
                    "baseline_us": base_val,
                }
            )
        elif delta_pct > warn_pct:
            findings.append(
                {
                    "phase": phase,
                    "severity": "warning",
                    "delta_pct": round(delta_pct, 1),
                    "current_us": curr_val,
                    "baseline_us": base_val,
                }
            )
        elif delta_pct < -warn_pct:
            findings.append(
                {
                    "phase": phase,
                    "severity": "improvement",
                    "delta_pct": round(delta_pct, 1),
                    "current_us": curr_val,
                    "baseline_us": base_val,
                }
            )

    return findings


# ── Bottleneck Analysis ──────────────────────────────────────────────────────


def identify_bottlenecks(phases, top_n=3):
    """Identify the top N phases by percentage of total time."""
    total = phases.get("total", {}).get("p50", 0)
    if total == 0:
        return []

    bottlenecks = []
    for phase, data in phases.items():
        if phase == "total":
            continue
        p50 = data.get("p50", 0)
        pct = p50 / total * 100
        bottlenecks.append(
            {
                "phase": phase,
                "p50_us": p50,
                "pct_of_total": round(pct, 1),
            }
        )

    bottlenecks.sort(key=lambda x: x["p50_us"], reverse=True)
    return bottlenecks[:top_n]


# ── Report Generation ────────────────────────────────────────────────────────

SEVERITY_ICONS = {
    "alert": "[ALERT]",
    "warning": "[WARN]",
    "improvement": "[OK]",
}


def format_console_report(bench_data, perf_stats, regressions, bottlenecks):
    """Format a human-readable console report."""
    lines = []
    lines.append("=" * 70)
    lines.append("  NANVIX BENCHMARK ANALYSIS REPORT")
    lines.append("=" * 70)
    lines.append("")

    # Summary
    if bench_data.get("summary"):
        lines.append("## Summary")
        for key, val in bench_data["summary"].items():
            lines.append(f"  {key}: {val:,} us")
        lines.append("")

    # Phase breakdown from bench text output
    if bench_data.get("phases"):
        lines.append("## Phase Breakdown (from benchmark output)")
        lines.append(
            f"  {'Phase':<24} {'p50 (us)':>10} {'p95 (us)':>10} {'p99 (us)':>10}"
        )
        lines.append("  " + "-" * 56)
        for phase, data in bench_data["phases"].items():
            lines.append(
                f"  {phase:<24} {data['p50']:>10,} {data['p95']:>10,} {data['p99']:>10,}"
            )
        lines.append("")

    # Distribution analysis from PERF_TIMINGS
    if perf_stats:
        lines.append("## Per-Phase Distribution (from PERF_TIMINGS)")
        lines.append(
            f"  {'Phase':<24} {'mean':>8} {'stddev':>8} {'CV%':>6} {'min':>8} {'max':>8} {'n':>4}"
        )
        lines.append("  " + "-" * 70)
        for phase, stats in perf_stats.items():
            lines.append(
                f"  {phase:<24} {stats['mean']:>8,.0f} {stats['stddev']:>8,.0f}"
                f" {stats['cv_pct']:>5.1f}% {stats['min']:>8,} {stats['max']:>8,}"
                f" {stats['count']:>4}"
            )
        lines.append("")

    # Bottlenecks
    if bottlenecks:
        lines.append("## Top Bottlenecks")
        for i, b in enumerate(bottlenecks, 1):
            lines.append(
                f"  {i}. {b['phase']}: {b['p50_us']:,} us ({b['pct_of_total']:.1f}% of total)"
            )
        lines.append("")

    # Regressions
    if regressions:
        lines.append("## Regression Analysis")
        for r in regressions:
            icon = SEVERITY_ICONS.get(r["severity"], "")
            direction = "slower" if r["delta_pct"] > 0 else "faster"
            lines.append(
                f"  {icon} {r['phase']}: {abs(r['delta_pct']):.1f}% {direction}"
                f" ({r['current_us']:,} vs {r['baseline_us']:,} us)"
            )
        lines.append("")
    elif regressions is not None:
        lines.append("## Regression Analysis")
        lines.append("  No significant regressions detected.")
        lines.append("")

    lines.append("=" * 70)
    return "\n".join(lines)


def build_json_report(bench_data, perf_stats, regressions, bottlenecks, iterations):
    """Build a JSON-serializable report dict."""
    report = {
        "summary": bench_data.get("summary", {}),
        "phases": bench_data.get("phases", {}),
        "bottlenecks": bottlenecks or [],
    }

    if perf_stats:
        report["distribution"] = perf_stats

    if regressions is not None:
        report["regressions"] = regressions

    if iterations:
        report["iteration_count"] = len(iterations)

    return report


# ── Write PERF_TIMINGS CSV ───────────────────────────────────────────────────


def write_perf_csv(iterations, output_path):
    """Write per-iteration PERF_TIMINGS data to CSV."""
    if not iterations:
        return

    # Collect all phase names in order
    all_phases = []
    seen = set()
    for it in iterations:
        for phase in it:
            if phase not in seen:
                all_phases.append(phase)
                seen.add(phase)

    with open(output_path, "w") as f:
        f.write("iteration," + ",".join(all_phases) + "\n")
        for i, it in enumerate(iterations):
            vals = [str(it.get(phase, "")) for phase in all_phases]
            f.write(f"{i}," + ",".join(vals) + "\n")


# ── Main ─────────────────────────────────────────────────────────────────────


def cmd_report(args):
    """Generate analysis report from benchmark output files."""

    # Parse stdout (bench text table)
    bench_data = {"summary": {}, "phases": OrderedDict()}
    if args.stdout and os.path.isfile(args.stdout):
        with open(args.stdout, "r") as f:
            bench_data = parse_bench_stdout(f.read())

    # Parse stderr (PERF_TIMINGS)
    iterations = []
    perf_stats = {}
    if args.stderr and os.path.isfile(args.stderr):
        with open(args.stderr, "r") as f:
            iterations = parse_perf_timings(f.read())
        if iterations:
            perf_stats = compute_phase_distribution(iterations)

    # Use PERF_TIMINGS phases if bench text didn't have them
    phases_for_analysis = bench_data.get("phases", {})
    if not phases_for_analysis and perf_stats:
        phases_for_analysis = {
            phase: {"p50": stats["p50"], "p95": stats["p95"], "p99": stats["p99"]}
            for phase, stats in perf_stats.items()
        }

    # Bottleneck analysis
    bottlenecks = identify_bottlenecks(phases_for_analysis)

    # Regression detection
    regressions = None
    if args.baseline and os.path.isfile(args.baseline):
        baseline_phases = parse_baseline_csv(args.baseline)
        if baseline_phases:
            regressions = detect_regressions(phases_for_analysis, baseline_phases)

    # Output
    console_report = format_console_report(
        bench_data, perf_stats, regressions, bottlenecks
    )
    print(console_report)

    if args.json:
        report = build_json_report(
            bench_data, perf_stats, regressions, bottlenecks, iterations
        )
        with open(args.json, "w") as f:
            json.dump(report, f, indent=2)
        print(f"JSON report written to: {args.json}")

    if args.perf_csv and iterations:
        write_perf_csv(iterations, args.perf_csv)
        print(f"PERF_TIMINGS CSV written to: {args.perf_csv}")

    # Exit with error if there are alerts
    if regressions:
        alerts = [r for r in regressions if r["severity"] == "alert"]
        if alerts:
            print(f"\n{len(alerts)} phase(s) regressed >10%. See report above.")
            return 1

    return 0


def cmd_compare(args):
    """Compare two benchmark runs side-by-side (A/B comparison).

    Reads PERF_TIMINGS from two stderr files and produces a delta report
    showing which phases improved, regressed, or stayed the same.
    """
    # Parse both runs
    for label, path in [("before", args.before), ("after", args.after)]:
        if not os.path.isfile(path):
            print(f"ERROR: {label} file not found: {path}", file=sys.stderr)
            return 1

    with open(args.before, "r") as f:
        before_iters = parse_perf_timings(f.read())
    with open(args.after, "r") as f:
        after_iters = parse_perf_timings(f.read())

    if not before_iters:
        print("ERROR: No PERF_TIMINGS found in before file.", file=sys.stderr)
        return 1
    if not after_iters:
        print("ERROR: No PERF_TIMINGS found in after file.", file=sys.stderr)
        return 1

    before_stats = compute_phase_distribution(before_iters)
    after_stats = compute_phase_distribution(after_iters)

    # Merge phase names (preserve order from before, then any new in after)
    all_phases = list(before_stats.keys())
    for p in after_stats:
        if p not in before_stats:
            all_phases.append(p)

    lines = []
    lines.append("=" * 82)
    lines.append("  NANVIX BENCHMARK COMPARISON (A/B)")
    lines.append("=" * 82)
    lines.append(f"  Before: {args.before} ({len(before_iters)} iterations)")
    lines.append(f"  After:  {args.after} ({len(after_iters)} iterations)")
    lines.append("")

    # Phase-by-phase comparison table
    lines.append(
        f"  {'Phase':<24} {'Before p50':>10} {'After p50':>10} {'Delta':>10} {'%':>7}  Status"
    )
    lines.append("  " + "-" * 78)

    has_regression = False
    for phase in all_phases:
        b = before_stats.get(phase, {})
        a = after_stats.get(phase, {})
        bp50 = b.get("p50", 0)
        ap50 = a.get("p50", 0)

        if bp50 == 0 and ap50 == 0:
            continue

        delta = ap50 - bp50
        pct = (delta / bp50 * 100) if bp50 > 0 else 0

        if pct < -5:
            status = "[OK] faster"
        elif pct > 10:
            status = "[ALERT] slower"
            has_regression = True
        elif pct > 5:
            status = "[WARN] slower"
        else:
            status = ""

        lines.append(
            f"  {phase:<24} {bp50:>10,} {ap50:>10,} {delta:>+10,} {pct:>+6.1f}%  {status}"
        )

    lines.append("")

    # Variability comparison
    lines.append("  Variability (CV%):")
    lines.append(f"  {'Phase':<24} {'Before CV%':>10} {'After CV%':>10} {'Change':>10}")
    lines.append("  " + "-" * 58)

    for phase in all_phases:
        b = before_stats.get(phase, {})
        a = after_stats.get(phase, {})
        bcv = b.get("cv_pct", 0)
        acv = a.get("cv_pct", 0)
        if bcv == 0 and acv == 0:
            continue
        cv_delta = acv - bcv
        marker = ""
        if acv > 15 and bcv <= 15:
            marker = "  [!] now noisy"
        elif bcv > 15 and acv <= 15:
            marker = "  [OK] stabilized"
        lines.append(
            f"  {phase:<24} {bcv:>9.1f}% {acv:>9.1f}% {cv_delta:>+9.1f}%{marker}"
        )

    lines.append("")

    # Outlier analysis for the "after" run
    if len(after_iters) >= 10:
        total_key = "total" if "total" in after_stats else None
        if total_key:
            total_vals = [it.get(total_key, 0) for it in after_iters if total_key in it]
            if total_vals:
                stats = compute_stats(total_vals)
                mean = stats["mean"]
                stddev = stats["stddev"]
                if stddev > 0:
                    outliers = []
                    for i, v in enumerate(total_vals):
                        z = abs(v - mean) / stddev
                        if z > 2.5:
                            outliers.append((i, v, z))
                    if outliers:
                        lines.append("  Outlier iterations (total, |z| > 2.5):")
                        for idx, val, z in outliers[:5]:
                            lines.append(
                                f"    iteration {idx}: {val:,} us (z={z:+.1f})"
                            )
                        lines.append("")

    lines.append("=" * 82)
    print("\n".join(lines))

    if args.json:
        report = {
            "before_file": args.before,
            "after_file": args.after,
            "before_iterations": len(before_iters),
            "after_iterations": len(after_iters),
            "phases": {},
        }
        for phase in all_phases:
            b = before_stats.get(phase, {})
            a = after_stats.get(phase, {})
            bp50 = b.get("p50", 0)
            ap50 = a.get("p50", 0)
            delta = ap50 - bp50
            pct = (delta / bp50 * 100) if bp50 > 0 else 0
            report["phases"][phase] = {
                "before_p50": bp50,
                "after_p50": ap50,
                "delta_us": delta,
                "delta_pct": round(pct, 1),
                "before_cv": b.get("cv_pct", 0),
                "after_cv": a.get("cv_pct", 0),
            }
        with open(args.json, "w") as f:
            json.dump(report, f, indent=2)
        print(f"JSON comparison written to: {args.json}")

    return 1 if has_regression else 0


def main():
    parser = argparse.ArgumentParser(
        description="Nanvix Benchmark Analysis Tool",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(dest="command", help="Command to run")

    # report command
    report_parser = subparsers.add_parser("report", help="Generate analysis report")
    report_parser.add_argument(
        "--stdout", required=True, help="Path to benchmark stdout file"
    )
    report_parser.add_argument(
        "--stderr", help="Path to benchmark stderr file (with PERF_TIMINGS)"
    )
    report_parser.add_argument(
        "--baseline", help="Path to baseline CSV for regression detection"
    )
    report_parser.add_argument("--json", help="Write JSON report to this path")
    report_parser.add_argument(
        "--perf-csv", help="Write per-iteration PERF_TIMINGS to CSV"
    )

    # compare command
    compare_parser = subparsers.add_parser(
        "compare", help="Compare two benchmark runs (A/B)"
    )
    compare_parser.add_argument(
        "--before",
        required=True,
        help="Path to before-run stderr file (with PERF_TIMINGS)",
    )
    compare_parser.add_argument(
        "--after",
        required=True,
        help="Path to after-run stderr file (with PERF_TIMINGS)",
    )
    compare_parser.add_argument("--json", help="Write JSON comparison to this path")

    args = parser.parse_args()

    if args.command == "report":
        sys.exit(cmd_report(args))
    elif args.command == "compare":
        sys.exit(cmd_compare(args))
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
