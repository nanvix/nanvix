#!/usr/bin/env python3
"""
Nanvix ETL Trace Analyzer
==========================

Parses xperf text dumps from WPR/ETW traces to produce actionable summaries
without requiring WPA. Extracts CPU sampling, context-switch, and scheduling
data scoped to the benchmark process (nanvix-bench.exe and its children).

Usage:
    # Full analysis (recommended)
    python analyze-etl.py D:\\traces\\cold-start.etl

    # With symbol resolution (resolves kernel function names)
    python analyze-etl.py D:\\traces\\cold-start.etl --symbols

    # Only CPU profile (fastest)
    python analyze-etl.py D:\\traces\\cold-start.etl --sections cpu

    # JSON output
    python analyze-etl.py D:\\traces\\cold-start.etl --json report.json

    # Custom process filter
    python analyze-etl.py D:\\traces\\cold-start.etl --process nanvixd.exe

    # Generate folded stacks for flamegraph.pl
    python analyze-etl.py D:\\traces\\cold-start.etl --folded stacks.folded
    perl FlameGraph/flamegraph.pl stacks.folded > flamegraph.svg

    # Kernel stack analysis (module/function breakdown + WHP call chains)
    python analyze-etl.py D:\\traces\\cold-start.etl --stacks
    python analyze-etl.py D:\\traces\\cold-start.etl --stacks --json stacks.json

Symbol Resolution:
    The --symbols flag requires _NT_SYMBOL_PATH and _NT_SYMCACHE_PATH env vars:
        set _NT_SYMBOL_PATH=srv*C:\\Symbols*https://msdl.microsoft.com/download/symbols
        set _NT_SYMCACHE_PATH=C:\\SymCache

Dependencies: Python 3.10+ (stdlib only), xperf (Windows Performance Toolkit)
Flamegraphs: https://github.com/brendangregg/FlameGraph (flamegraph.pl)
"""

import argparse
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from html.parser import HTMLParser

# ── xperf invocation ─────────────────────────────────────────────────────────

XPERF_PATHS = [
    r"C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe",
    r"C:\Program Files\Windows Kits\10\Windows Performance Toolkit\xperf.exe",
]


def find_xperf():
    """Locate xperf.exe on the system.

    Checks NANVIX_XPERF_PATH env var first, then well-known install paths,
    then PATH.
    """
    env_path = os.environ.get("NANVIX_XPERF_PATH")
    if env_path and os.path.isfile(env_path):
        return env_path
    for p in XPERF_PATHS:
        if os.path.isfile(p):
            return p
    # Try PATH
    found = shutil.which("xperf")
    if found:
        return found
    return None


def run_xperf_merge(etl_path, output_path=None, xperf_path=None):
    """Run xperf -merge to prepare an ETL for symbol resolution.

    Merging embeds image identification info into the trace so that
    xperf -symbols can resolve kernel function names later.

    Args:
        etl_path: Path to the raw .etl trace file.
        output_path: Path for the merged output .etl. Defaults to
            replacing the extension with '-merged.etl'.
        xperf_path: Optional explicit path to xperf.exe.

    Returns:
        Path to the merged ETL file.
    """
    xperf = xperf_path or find_xperf()
    if not xperf:
        print(
            "ERROR: xperf.exe not found. Install Windows Performance Toolkit.",
            file=sys.stderr,
        )
        sys.exit(1)

    if output_path is None:
        base, _ = os.path.splitext(etl_path)
        output_path = f"{base}-merged.etl"

    print(f"[etl] Merging {etl_path} -> {output_path}", file=sys.stderr)
    cmd = [xperf, "-merge", etl_path, output_path]

    result = subprocess.run(
        cmd, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    if result.returncode != 0:
        stderr_text = result.stderr[:500] if result.stderr else ""
        if "Events were lost" in stderr_text or "Events were lost" in (
            result.stdout or ""
        ):
            print(
                f"WARNING: xperf -merge reported lost events (exit {result.returncode}). "
                "Continuing with available data.",
                file=sys.stderr,
            )
        else:
            print(
                f"ERROR: xperf -merge failed (exit {result.returncode})",
                file=sys.stderr,
            )
            if result.stderr:
                print(stderr_text, file=sys.stderr)
            sys.exit(1)

    merged_size = os.path.getsize(output_path) / (1024 * 1024)
    print(f"[etl] Merged trace: {output_path} ({merged_size:.1f} MB)", file=sys.stderr)
    return output_path


def run_xperf_butterfly(etl_path, process_filter="nanvix", min_hits=5, xperf_path=None):
    """Run xperf -a stack -butterfly and return the HTML output.

    Args:
        etl_path: Path to a merged .etl trace file.
        process_filter: Regex to filter processes (default: "nanvix").
        min_hits: Minimum hit count for inclusion (default: 5).
        xperf_path: Optional explicit path to xperf.exe.

    Returns:
        HTML string from the butterfly report.
    """
    xperf = xperf_path or find_xperf()
    if not xperf:
        print(
            "ERROR: xperf.exe not found. Install Windows Performance Toolkit.",
            file=sys.stderr,
        )
        sys.exit(1)

    sym_path = os.environ.get("_NT_SYMBOL_PATH", "")
    if not sym_path:
        print(
            "WARNING: _NT_SYMBOL_PATH not set -- kernel symbols will show "
            "as raw addresses.",
            file=sys.stderr,
        )

    # -o must precede -a per xperf syntax
    with tempfile.NamedTemporaryFile(suffix=".html", delete=False, mode="w") as tmp:
        out_path = tmp.name

    print(
        f"[etl] Running butterfly stack analysis (min_hits={min_hits}, "
        f"process={process_filter})...",
        file=sys.stderr,
    )

    cmd = [
        xperf,
        "-i",
        etl_path,
        "-o",
        out_path,
        "-symbols",
        "-a",
        "stack",
        "-butterfly",
        str(min_hits),
        "-process",
        process_filter,
    ]

    result = subprocess.run(
        cmd, capture_output=True, text=True, encoding="utf-8", errors="replace"
    )
    if result.returncode != 0:
        print(
            f"ERROR: xperf -a stack -butterfly failed (exit " f"{result.returncode})",
            file=sys.stderr,
        )
        if result.stderr:
            print(result.stderr[:500], file=sys.stderr)
        os.unlink(out_path)
        sys.exit(1)

    with open(out_path, "r", encoding="utf-8", errors="replace") as f:
        html = f.read()
    os.unlink(out_path)

    print(f"[etl] Butterfly report: {len(html)} bytes", file=sys.stderr)
    return html


# ── Butterfly HTML parser ────────────────────────────────────────────────────

# WHP-related module/function prefixes for filtering hypervisor call chains.
_WHP_KEYWORDS = (
    "whv",
    "vid",
    "winhvr",
    "hypercall",
    "hypervisor",
    "partition",
    "winhv",
)


class _ButterflyHTMLParser(HTMLParser):
    """Parse xperf butterfly stack HTML into structured section tables."""

    def __init__(self):
        super().__init__()
        self._in_h2 = False
        self._in_td = False
        self._in_th = False
        self._cell_text = ""
        self._current_row = []
        self._row_class = ""
        self._current_section = ""
        self.sections = {}  # section_name -> list of (class, [cells])
        self._current_rows = []

    def handle_starttag(self, tag, attrs):
        attrs_d = dict(attrs)
        if tag == "h2":
            self._in_h2 = True
            self._cell_text = ""
        elif tag == "td":
            self._in_td = True
            self._cell_text = ""
        elif tag == "th":
            self._in_th = True
            self._cell_text = ""
        elif tag == "tr":
            self._current_row = []
            self._row_class = attrs_d.get("class", "")

    def handle_endtag(self, tag):
        if tag == "h2":
            self._in_h2 = False
            self._current_section = self._cell_text.strip()
            self._current_rows = []
            self.sections[self._current_section] = self._current_rows
        elif tag == "td":
            self._in_td = False
            self._current_row.append(self._cell_text.strip())
        elif tag == "th":
            self._in_th = False
            self._current_row.append(self._cell_text.strip())
        elif tag == "tr":
            if self._current_row and self._current_section:
                self._current_rows.append((self._row_class, self._current_row))

    def handle_data(self, data):
        if self._in_td or self._in_th or self._in_h2:
            self._cell_text += data

    def handle_entityref(self, name):
        if self._in_td or self._in_th or self._in_h2:
            if name == "nbsp":
                self._cell_text += " "


def parse_butterfly_html(html):
    """Parse xperf butterfly HTML into structured data.

    Returns a dict with:
        modules_exclusive: [(name, hits, pct), ...]
        functions_exclusive: [(name, hits, pct), ...]
        functions_inclusive: [(name, hits, pct), ...]
        whp_functions: [(name, hits, pct, detail), ...]
    """
    parser = _ButterflyHTMLParser()
    parser.feed(html)

    result = {
        "modules_exclusive": [],
        "functions_exclusive": [],
        "functions_inclusive": [],
        "whp_functions": [],
    }

    # Modules by Exclusive Hits
    for _cls, row in parser.sections.get("Modules by Exclusive Hits", []):
        if len(row) >= 3 and row[1].isdigit():
            result["modules_exclusive"].append(
                {
                    "module": row[0],
                    "hits": int(row[1]),
                    "percent": row[2],
                }
            )

    # Functions by Exclusive Hits
    for _cls, row in parser.sections.get("Functions by Exclusive Hits", []):
        if len(row) >= 3 and row[1].isdigit():
            result["functions_exclusive"].append(
                {
                    "function": row[0],
                    "hits": int(row[1]),
                    "percent": row[2],
                }
            )

    # Functions by UniInclusive Hits
    for _cls, row in parser.sections.get("Functions by UniInclusive Hits", []):
        if len(row) >= 3 and row[1].isdigit():
            result["functions_inclusive"].append(
                {
                    "function": row[0],
                    "hits": int(row[1]),
                    "percent": row[2],
                }
            )

    # WHP-related functions from Multi-Inclusive section
    for _cls, row in parser.sections.get(
        "Functions by Multi-Inclusive Hits with Callers and Callees", []
    ):
        if len(row) >= 2:
            name_lower = row[0].lower()
            if any(kw in name_lower for kw in _WHP_KEYWORDS):
                result["whp_functions"].append(
                    {
                        "function": row[0],
                        "hits": row[1] if len(row) > 1 else "",
                        "percent": row[2] if len(row) > 2 else "",
                        "detail": row[3] if len(row) > 3 else "",
                    }
                )

    return result


def format_stacks_report(stacks_data):
    """Format parsed butterfly data into a human-readable report."""
    lines = []

    # Module breakdown
    lines.append("")
    lines.append("=" * 70)
    lines.append("  MODULES BY EXCLUSIVE CPU HITS")
    lines.append("=" * 70)
    for m in stacks_data["modules_exclusive"]:
        bar_len = 0
        try:
            bar_len = int(float(m["percent"].rstrip("%")) * 0.5)
        except (ValueError, AttributeError):
            pass
        bar = "#" * bar_len
        lines.append(
            f"  {m['module']:40s} {m['hits']:>7d} ({m['percent']:>7s}) " f"{bar}"
        )

    # Top functions (exclusive)
    lines.append("")
    lines.append("=" * 70)
    lines.append("  TOP FUNCTIONS BY EXCLUSIVE CPU HITS")
    lines.append("=" * 70)
    for f in stacks_data["functions_exclusive"][:30]:
        lines.append(f"  {f['function']:60s} {f['hits']:>7d} ({f['percent']:>7s})")

    # Top functions (inclusive -- call chain attribution)
    lines.append("")
    lines.append("=" * 70)
    lines.append("  TOP FUNCTIONS BY INCLUSIVE CPU HITS (call chain)")
    lines.append("=" * 70)
    for f in stacks_data["functions_inclusive"][:30]:
        lines.append(f"  {f['function']:60s} {f['hits']:>7d} ({f['percent']:>7s})")

    # WHP / hypervisor functions
    whp = stacks_data["whp_functions"]
    if whp:
        lines.append("")
        lines.append("=" * 70)
        lines.append("  WHP / HYPERVISOR FUNCTIONS")
        lines.append("=" * 70)
        # Group by top-level WHP API calls
        api_calls = []
        callers_callees = []
        for f in whp:
            name = f["function"]
            if name.startswith("<--") or name.startswith("-->"):
                callers_callees.append(f)
            else:
                api_calls.append(f)

        for f in api_calls:
            lines.append(f"  {f['function']:55s} {f['hits']:>6s} {f['percent']:>8s}")
        if callers_callees:
            lines.append("")
            lines.append("  Call chain details (top 20):")
            for f in callers_callees[:20]:
                lines.append(
                    f"    {f['function']:53s} {f['hits']:>6s} " f"{f['percent']:>8s}"
                )

    # Analysis summary
    lines.append("")
    lines.append("=" * 70)
    lines.append("  ANALYSIS SUMMARY")
    lines.append("=" * 70)

    modules = stacks_data["modules_exclusive"]
    if modules:
        top_mod = modules[0]
        lines.append(
            f"  Dominant module: {top_mod['module']} "
            f"({top_mod['percent']} exclusive CPU)"
        )

    funcs = stacks_data["functions_exclusive"]
    if funcs:
        top_func = funcs[0]
        lines.append(
            f"  Top hotspot:     {top_func['function']} "
            f"({top_func['percent']} exclusive CPU)"
        )

    # Compute WHP API total
    whp_api_hits = (
        sum(int(f["hits"]) for f in api_calls if f["hits"].isdigit()) if whp else 0
    )
    total_hits = sum(m["hits"] for m in modules) if modules else 1
    if whp_api_hits:
        whp_pct = whp_api_hits / total_hits * 100
        lines.append(f"  WHP API total:   {whp_api_hits} hits ({whp_pct:.1f}% of CPU)")

    lines.append("")
    return "\n".join(lines)


def run_xperf_dump(etl_path, xperf_path=None, symbols=False, stacks=False):
    """Run xperf -a dumper and return the text output.

    Args:
        etl_path: Path to the .etl trace file.
        xperf_path: Optional explicit path to xperf.exe.
        symbols: If True, enable symbol resolution via -symbols flag.
                 Requires _NT_SYMBOL_PATH and _NT_SYMCACHE_PATH env vars.
        stacks: If True, add -stacktimeshifting to combine stack fragments.
    """
    xperf = xperf_path or find_xperf()
    if not xperf:
        print(
            "ERROR: xperf.exe not found. Install Windows Performance Toolkit.",
            file=sys.stderr,
        )
        sys.exit(1)

    if symbols:
        sym_path = os.environ.get("_NT_SYMBOL_PATH", "")
        if not sym_path:
            print(
                "WARNING: --symbols requested but _NT_SYMBOL_PATH not set.",
                file=sys.stderr,
            )
            print(
                "  Set it to e.g.: srv*C:\\Symbols*https://msdl.microsoft.com/download/symbols",
                file=sys.stderr,
            )
        print(
            f"[etl] Parsing {etl_path} with xperf (symbol resolution ON)...",
            file=sys.stderr,
        )
    else:
        print(f"[etl] Parsing {etl_path} with xperf...", file=sys.stderr)
    print("[etl] This may take a moment for large traces.", file=sys.stderr)

    # Build command: -symbols must come BEFORE -a dumper
    cmd = [xperf, "-i", etl_path]
    if symbols:
        cmd.append("-symbols")
    cmd.extend(["-a", "dumper"])
    if stacks:
        cmd.append("-stacktimeshifting")

    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )

    if result.returncode != 0:
        stderr_text = result.stderr[:500] if result.stderr else ""
        # "Events were lost" is a non-fatal warning — output is still usable.
        if "Events were lost" in stderr_text or "Events were lost" in (
            result.stdout or ""
        ):
            print(
                f"WARNING: xperf reported lost events (exit {result.returncode}). "
                "Data may be slightly incomplete but is still usable.",
                file=sys.stderr,
            )
        else:
            print(f"ERROR: xperf failed (exit {result.returncode})", file=sys.stderr)
            if result.stderr:
                print(stderr_text, file=sys.stderr)
            sys.exit(1)

    # Filter out progress bars (lines with [1/2] etc.)
    lines = []
    for line in result.stdout.split("\n"):
        if re.match(r"^\[?\d+/\d+\]", line.strip()):
            continue
        lines.append(line)

    return "\n".join(lines)


# ── Parsing ──────────────────────────────────────────────────────────────────


def parse_events(text, target_processes):
    """Parse xperf dump text into structured events.

    Returns dict with:
        - cpu_samples: list of (timestamp, pid, tid, module, function)
        - cswitches: list of (timestamp, new_pid, new_process, old_pid, old_process, new_tid)
        - ready_threads: list of (timestamp, readied_pid, readied_process, readied_tid)
        - process_lifetimes: dict of pid -> (name, start_ts, end_ts)
        - thread_counts: dict of pid -> set of tids
    """
    target_set = {p.lower() for p in target_processes}

    cpu_samples = []
    cswitches = []
    ready_threads = []
    process_lifetimes = {}  # pid -> (name, start_ts, end_ts)
    thread_counts = defaultdict(set)

    # Regex patterns for common events
    # SampledProfile format:
    #   timestamp, process (pid), tid, ip, cpu, user_frame, kernel_frame, count, type
    re_sampled = re.compile(
        r"^\s*SampledProfile,\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),"
        r"\s+(\d+),\s+\S+,\s+\d+,\s+(.+?),\s+(.+?),\s+\d+"
    )

    # CSwitch: timestamp, new_process (new_pid), new_tid, ...  old_process (old_pid), old_tid, ...
    re_cswitch = re.compile(r"^\s*CSwitch,\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),\s+(\d+),")
    # For old process in CSwitch line (kept for future CSwitch analysis)
    _re_cswitch_old = re.compile(  # noqa: F841
        r"(.+?)\s+\(\s*(\d+)\),\s+(\d+),\s+(\d+),\s+\d+,\s+(\S+),"
    )

    # ReadyThread: timestamp, readying_process (pid), tid, readied_process (pid), readied_tid, ...
    re_ready = re.compile(
        r"^\s*ReadyThread,\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),"
        r"\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),\s+(\d+),"
    )

    # Process start/end
    re_pstart = re.compile(r"^\s*P-(?:Start|DCStart),\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),")
    re_pend = re.compile(r"^\s*P-(?:End|DCEnd),\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),")

    for line in text.split("\n"):
        stripped = line.strip()

        # Process start
        m = re_pstart.match(stripped)
        if m:
            ts, name, pid = int(m.group(1)), m.group(2).strip(), int(m.group(3))
            process_lifetimes[pid] = (name, ts, None)
            continue

        # Process end
        m = re_pend.match(stripped)
        if m:
            ts, name, pid = int(m.group(1)), m.group(2).strip(), int(m.group(3))
            if pid in process_lifetimes:
                old = process_lifetimes[pid]
                process_lifetimes[pid] = (old[0], old[1], ts)
            else:
                process_lifetimes[pid] = (name, None, ts)
            continue

        # CPU sampling - check if it involves our target process
        m = re_sampled.match(stripped)
        if m:
            ts = int(m.group(1))
            proc_name = m.group(2).strip()
            pid = int(m.group(3))
            tid = int(m.group(4))
            user_frame = m.group(5).strip()
            kernel_frame = m.group(6).strip()

            if proc_name.lower() in target_set:
                # Use the kernel frame if available, otherwise user frame
                module_func = (
                    kernel_frame if kernel_frame and "!" in kernel_frame else user_frame
                )
                cpu_samples.append((ts, pid, tid, proc_name, module_func))
                thread_counts[pid].add(tid)
            continue

        # Context switches
        m = re_cswitch.match(stripped)
        if m:
            ts = int(m.group(1))
            new_proc = m.group(2).strip()
            new_pid = int(m.group(3))
            new_tid = int(m.group(4))

            if new_proc.lower() in target_set:
                # CSwitch format after new_tid:
                #   new_cpu, new_pri, wait_time, quantum_used, old_process (old_pid), old_tid, ...
                remaining = stripped[m.end() :]
                old_proc = "unknown"
                old_pid = 0
                # Skip 4 numeric fields, then match "process_name (pid)"
                old_m = re.search(
                    r"(?:\s*-?\d+\s*,){4}\s*(.+?)\s+\(\s*(\d+)\)",
                    remaining,
                )
                if old_m:
                    old_proc = old_m.group(1).strip()
                    old_pid = int(old_m.group(2))

                cswitches.append((ts, new_pid, new_proc, old_pid, old_proc, new_tid))
                thread_counts[new_pid].add(new_tid)
            continue

        # Ready thread
        m = re_ready.match(stripped)
        if m:
            ts = int(m.group(1))
            readied_proc = m.group(5).strip()
            readied_pid = int(m.group(6))
            readied_tid = int(m.group(7))

            if readied_proc.lower() in target_set:
                ready_threads.append((ts, readied_pid, readied_proc, readied_tid))
            continue

    return {
        "cpu_samples": cpu_samples,
        "cswitches": cswitches,
        "ready_threads": ready_threads,
        "process_lifetimes": process_lifetimes,
        "thread_counts": dict(thread_counts),
    }


# ── Analysis ─────────────────────────────────────────────────────────────────


def analyze_cpu_samples(samples):
    """Analyze CPU sampling data for hot functions and modules.

    Returns aggregate stats plus a per-process breakdown with independent
    module/function rankings.
    """
    if not samples:
        return {"total_samples": 0}

    module_counts = Counter()
    function_counts = Counter()

    # Per-process counters
    per_process_modules = defaultdict(Counter)
    per_process_functions = defaultdict(Counter)
    per_process_total = Counter()

    for ts, pid, tid, proc, mod_func in samples:
        if "!" in mod_func:
            module, func = mod_func.split("!", 1)
            module = module.strip()
            func = func.strip()
        else:
            module = mod_func.strip()
            func = "<unknown>"

        module_counts[module] += 1
        function_counts[mod_func] += 1

        per_process_modules[proc][module] += 1
        per_process_functions[proc][mod_func] += 1
        per_process_total[proc] += 1

    total = len(samples)

    # Top modules
    top_modules = [
        {"module": mod, "samples": count, "pct": round(count / total * 100, 1)}
        for mod, count in module_counts.most_common(10)
    ]

    # Top functions
    top_functions = [
        {"function": func, "samples": count, "pct": round(count / total * 100, 1)}
        for func, count in function_counts.most_common(20)
    ]

    # Kernel vs user split
    kernel_modules = {
        "ntoskrnl.exe",
        "ntdll.dll",
        "win32kfull.sys",
        "hal.dll",
        "fltmgr.sys",
        "ci.dll",
        "nt",
        "winhvr.sys",
        "hvax64.exe",
        "winhv.sys",
        "kd.dll",
        "ksecdd.sys",
        "cng.sys",
    }
    kernel_samples = sum(
        c for mod, c in module_counts.items() if mod.lower() in kernel_modules
    )

    # Per-process breakdown
    per_process = {}
    for proc in sorted(per_process_total, key=per_process_total.get, reverse=True):
        proc_total = per_process_total[proc]
        proc_kernel = sum(
            c
            for mod, c in per_process_modules[proc].items()
            if mod.lower() in kernel_modules
        )
        per_process[proc] = {
            "total_samples": proc_total,
            "pct_of_total": round(proc_total / total * 100, 1),
            "kernel_pct": (
                round(proc_kernel / proc_total * 100, 1) if proc_total > 0 else 0
            ),
            "top_modules": [
                {
                    "module": mod,
                    "samples": count,
                    "pct": round(count / proc_total * 100, 1),
                }
                for mod, count in per_process_modules[proc].most_common(10)
            ],
            "top_functions": [
                {
                    "function": func,
                    "samples": count,
                    "pct": round(count / proc_total * 100, 1),
                }
                for func, count in per_process_functions[proc].most_common(15)
            ],
        }

    return {
        "total_samples": total,
        "top_modules": top_modules,
        "top_functions": top_functions,
        "kernel_samples": kernel_samples,
        "user_samples": total - kernel_samples,
        "kernel_pct": round(kernel_samples / total * 100, 1) if total > 0 else 0,
        "per_process": per_process,
    }


def analyze_context_switches(cswitches, process_lifetimes):
    """Analyze context switch patterns."""
    if not cswitches:
        return {"total_cswitches": 0}

    total = len(cswitches)

    # Compute time on CPU from consecutive switches
    # Group by thread
    by_thread = defaultdict(list)
    for ts, new_pid, new_proc, old_pid, old_proc, new_tid in cswitches:
        by_thread[new_tid].append(ts)

    # Calculate inter-switch intervals
    intervals = []
    for tid, timestamps in by_thread.items():
        timestamps.sort()
        for i in range(1, len(timestamps)):
            delta = timestamps[i] - timestamps[i - 1]
            if 0 < delta < 10_000_000:  # filter unreasonable deltas (>10s)
                intervals.append(delta)

    stats = {}
    if intervals:
        intervals.sort()
        n = len(intervals)
        stats = {
            "switch_interval_p50_us": round(
                intervals[min(math.ceil(n * 0.50) - 1, n - 1)] / 10, 1
            ),
            "switch_interval_p95_us": round(
                intervals[min(math.ceil(n * 0.95) - 1, n - 1)] / 10, 1
            ),
            "switch_interval_p99_us": round(
                intervals[min(math.ceil(n * 0.99) - 1, n - 1)] / 10, 1
            ),
            "switch_interval_mean_us": round(sum(intervals) / n / 10, 1),
        }

    # Who preempted us?
    preempted_by = Counter()
    for ts, new_pid, new_proc, old_pid, old_proc, new_tid in cswitches:
        preempted_by[old_proc] += 1

    top_preemptors = [
        {"process": proc, "count": count, "pct": round(count / total * 100, 1)}
        for proc, count in preempted_by.most_common(10)
    ]

    return {
        "total_cswitches": total,
        "unique_threads": len(by_thread),
        "top_preemptors": top_preemptors,
        **stats,
    }


def analyze_scheduling(ready_threads):
    """Analyze thread scheduling latency."""
    if not ready_threads:
        return {"total_ready_events": 0}

    return {
        "total_ready_events": len(ready_threads),
    }


def analyze_process_timeline(process_lifetimes, target_processes):
    """Analyze process creation and lifecycle."""
    target_set = {p.lower() for p in target_processes}

    results = []
    for pid, (name, start_ts, end_ts) in process_lifetimes.items():
        if name.lower() not in target_set:
            continue
        duration_us = (
            round((end_ts - start_ts) / 10, 1) if start_ts and end_ts else None
        )
        results.append(
            {
                "pid": pid,
                "name": name,
                "start_ts": start_ts,
                "end_ts": end_ts,
                "duration_us": duration_us,
            }
        )

    results.sort(key=lambda x: x.get("start_ts") or 0)
    return results


# ── Flamegraph generation ────────────────────────────────────────────────────


def parse_folded_stacks(text, target_processes):
    """Parse xperf dump with -stacktimeshifting to extract folded call stacks.

    Returns a Counter mapping "frame1;frame2;...;frameN" -> sample count.
    Only includes SampledProfile events for target processes.
    """
    target_set = {p.lower() for p in target_processes}
    folded = Counter()

    re_sample = re.compile(
        r"^\s*SampledProfile,\s+(\d+),\s+(.+?)\s+\(\s*(\d+)\),\s+(\d+),"
    )
    re_stack = re.compile(r"^\s*Stack,\s+(\d+),\s+(\d+),\s+\d+,\s+\S+,\s+(.+)")

    current_sample = None  # (timestamp, tid, process_name)
    current_frames = []

    def flush_stack():
        nonlocal current_sample, current_frames
        if current_sample and current_frames:
            proc = current_sample[2]
            # Frames are top-of-stack first; reverse for flamegraph (root at left)
            frames = list(reversed(current_frames))
            key = f"{proc};{';'.join(frames)}"
            folded[key] += 1
        current_sample = None
        current_frames = []

    for line in text.split("\n"):
        stripped = line.strip()

        m = re_sample.match(stripped)
        if m:
            flush_stack()
            proc_name = m.group(2).strip()
            if proc_name.lower() in target_set:
                ts = int(m.group(1))
                tid = int(m.group(4))
                current_sample = (ts, tid, proc_name)
            continue

        if current_sample:
            m = re_stack.match(stripped)
            if m:
                ts = int(m.group(1))
                tid = int(m.group(2))
                if ts == current_sample[0] and tid == current_sample[1]:
                    func = m.group(3).strip().strip('"').replace('"', "")
                    if func and func != "Unknown":
                        current_frames.append(func)
                else:
                    flush_stack()
            elif stripped and not stripped.startswith("Stack,"):
                flush_stack()

    flush_stack()
    return folded


# ── Report formatting ────────────────────────────────────────────────────────


def format_report(cpu_analysis, cswitch_analysis, sched_analysis, timeline, sections):
    """Format human-readable console report."""
    lines = []
    lines.append("=" * 70)
    lines.append("  NANVIX ETL TRACE ANALYSIS")
    lines.append("=" * 70)
    lines.append("")

    # Process timeline
    if "timeline" in sections and timeline:
        lines.append("## Process Timeline")
        for p in timeline:
            dur = f"{p['duration_us']:,.0f} us" if p["duration_us"] else "N/A"
            lines.append(f"  PID {p['pid']:>6}  {p['name']:<30}  duration: {dur}")
        lines.append("")

    # CPU profile
    if "cpu" in sections and cpu_analysis.get("total_samples", 0) > 0:
        lines.append(f"## CPU Profile ({cpu_analysis['total_samples']} samples)")
        kernel_pct = cpu_analysis["kernel_pct"]
        lines.append(f"  Kernel: {kernel_pct}% | User: {100 - kernel_pct:.1f}%")
        lines.append("")

        lines.append("  Top modules:")
        lines.append(f"    {'Module':<40} {'Samples':>8} {'%':>6}")
        lines.append("    " + "-" * 56)
        for m in cpu_analysis["top_modules"]:
            lines.append(f"    {m['module']:<40} {m['samples']:>8} {m['pct']:>5.1f}%")
        lines.append("")

        lines.append("  Top functions:")
        lines.append(f"    {'Function':<56} {'Samples':>8} {'%':>6}")
        lines.append("    " + "-" * 72)
        for f in cpu_analysis["top_functions"][:15]:
            func_name = f["function"]
            if len(func_name) > 56:
                func_name = "..." + func_name[-53:]
            lines.append(f"    {func_name:<56} {f['samples']:>8} {f['pct']:>5.1f}%")
        lines.append("")

        # Per-process breakdown
        per_process = cpu_analysis.get("per_process", {})
        if len(per_process) > 1:
            for proc, pdata in per_process.items():
                lines.append(
                    f"  ### {proc} ({pdata['total_samples']} samples, "
                    f"{pdata['pct_of_total']}% of total, "
                    f"kernel: {pdata['kernel_pct']}%)"
                )
                lines.append("")

                lines.append(f"    {'Function':<56} {'Samples':>8} {'%':>6}")
                lines.append("    " + "-" * 72)
                for f in pdata["top_functions"][:10]:
                    func_name = f["function"]
                    if len(func_name) > 56:
                        func_name = "..." + func_name[-53:]
                    lines.append(
                        f"    {func_name:<56} {f['samples']:>8} {f['pct']:>5.1f}%"
                    )
                lines.append("")

    # Context switches
    if "cswitch" in sections and cswitch_analysis.get("total_cswitches", 0) > 0:
        total_cs = cswitch_analysis["total_cswitches"]
        unique_th = cswitch_analysis.get("unique_threads", 0)
        lines.append(f"## Context Switches ({total_cs} switches, {unique_th} threads)")

        if "switch_interval_p50_us" in cswitch_analysis:
            lines.append(
                f"  Switch interval: p50={cswitch_analysis['switch_interval_p50_us']} us"
                f"  p95={cswitch_analysis['switch_interval_p95_us']} us"
                f"  p99={cswitch_analysis['switch_interval_p99_us']} us"
                f"  mean={cswitch_analysis['switch_interval_mean_us']} us"
            )
        lines.append("")

        if cswitch_analysis.get("top_preemptors"):
            lines.append("  Switched from (who ran before us):")
            lines.append(f"    {'Process':<40} {'Count':>8} {'%':>6}")
            lines.append("    " + "-" * 56)
            for p in cswitch_analysis["top_preemptors"]:
                lines.append(
                    f"    {p['process']:<40} {p['count']:>8} {p['pct']:>5.1f}%"
                )
        lines.append("")

    # Scheduling
    if "sched" in sections and sched_analysis.get("total_ready_events", 0) > 0:
        lines.append(
            f"## Scheduling ({sched_analysis['total_ready_events']} ready events)"
        )
        lines.append("")

    # Summary insights
    lines.append("## Insights")
    insights = generate_insights(cpu_analysis, cswitch_analysis, timeline)
    if insights:
        for insight in insights:
            lines.append(f"  {insight}")
    else:
        lines.append("  No significant findings.")
    lines.append("")

    lines.append("=" * 70)
    return "\n".join(lines)


def generate_insights(cpu, cswitch, timeline):
    """Generate actionable insights from the analysis."""
    insights = []

    # CPU insights
    if cpu.get("total_samples", 0) > 0:
        if cpu["kernel_pct"] > 70:
            insights.append(
                f"[!] High kernel time ({cpu['kernel_pct']}%). "
                f"Most CPU is spent in kernel/hypervisor code."
            )
        elif cpu["kernel_pct"] > 50:
            insights.append(
                f"[i] Moderate kernel time ({cpu['kernel_pct']}%). "
                f"Consider reducing kernel calls or batching operations."
            )

        # Check for dominant module
        if cpu["top_modules"]:
            top = cpu["top_modules"][0]
            if top["pct"] > 40:
                insights.append(
                    f"[!] {top['module']} dominates CPU ({top['pct']}%). "
                    f"Focus optimization here."
                )

    # Context switch insights
    if cswitch.get("total_cswitches", 0) > 0:
        if cswitch["total_cswitches"] > 1000:
            insights.append(
                f"[!] High context switch count ({cswitch['total_cswitches']}). "
                f"Consider reducing preemption or pinning threads."
            )

        if cswitch.get("switch_interval_p50_us", 0) < 100:
            p50_us = cswitch["switch_interval_p50_us"]
            insights.append(
                f"[i] Very short scheduling intervals (p50={p50_us} us). "
                f"Threads may be busy-waiting or frequently yielding."
            )

    # Timeline insights
    if timeline:
        for p in timeline:
            if p["duration_us"] and p["name"].lower() == "nanvixd.exe":
                dur_ms = p["duration_us"] / 1000
                insights.append(
                    f"[*] nanvixd lifetime: {dur_ms:,.1f} ms (PID {p['pid']})"
                )

    return insights


# ── Main ─────────────────────────────────────────────────────────────────────

ALL_SECTIONS = {"cpu", "cswitch", "sched", "timeline"}


def main():
    parser = argparse.ArgumentParser(
        description="Analyze Nanvix benchmark ETL traces",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python analyze-etl.py traces\\cold-start.etl
  python analyze-etl.py traces\\cold-start.etl --symbols
  python analyze-etl.py traces\\cold-start.etl --symbols --sections cpu cswitch
  python analyze-etl.py traces\\cold-start.etl --json report.json
  python analyze-etl.py traces\\cold-start.etl --process nanvixd.exe
        """,
    )
    parser.add_argument("etl", help="Path to .etl trace file")
    parser.add_argument(
        "--sections",
        default=",".join(sorted(ALL_SECTIONS)),
        help="Comma-separated analysis sections: cpu, cswitch, sched, timeline (default: all)",
    )
    parser.add_argument(
        "--process",
        default="nanvix-bench.exe,nanvixd.exe",
        help="Comma-separated process names to analyze (default: nanvix-bench.exe,nanvixd.exe)",
    )
    parser.add_argument("--json", help="Write JSON report to this path")
    parser.add_argument("--xperf", help="Path to xperf.exe (auto-detected if omitted)")
    parser.add_argument(
        "--symbols",
        action="store_true",
        help="Enable symbol resolution (resolves hex addresses to function names). "
        "Requires _NT_SYMBOL_PATH and _NT_SYMCACHE_PATH environment variables.",
    )
    parser.add_argument(
        "--dump-file",
        help="Use pre-existing xperf dump text file instead of running xperf",
    )
    parser.add_argument(
        "--folded",
        help="Write folded stacks to this path (for use with Brendan Gregg's flamegraph.pl)",
    )
    parser.add_argument(
        "--stacks",
        action="store_true",
        help="Kernel stack analysis: merge ETL, run butterfly, and report "
        "module/function breakdown with WHP call chains. "
        "Implies --symbols.",
    )
    parser.add_argument(
        "--min-hits",
        type=int,
        default=5,
        help="Minimum hit count for butterfly stacks (default: 5). "
        "Only used with --stacks.",
    )

    args = parser.parse_args()

    # Parse comma-separated process list
    processes = [p.strip() for p in args.process.split(",") if p.strip()]

    # --folded implies --symbols
    if args.folded:
        args.symbols = True

    # --stacks implies --symbols and uses a different analysis path
    if args.stacks:
        args.symbols = True

    if not os.path.isfile(args.etl) and not args.dump_file:
        print(f"ERROR: ETL file not found: {args.etl}", file=sys.stderr)
        sys.exit(1)

    # ── Stacks mode: merge + butterfly + structured report ───────────────
    if args.stacks:
        # Build process filter for xperf -process (simple regex).
        # xperf's -process uses a basic regex; pipe (|) may not work.
        # Find common prefix, or use "nanvix" which matches both
        # nanvix-bench.exe and nanvixd.exe.
        proc_names = [p.replace(".exe", "") for p in processes]
        # Find longest common prefix
        if proc_names:
            prefix = os.path.commonprefix(proc_names)
            proc_regex = prefix if len(prefix) >= 3 else proc_names[0]
        else:
            proc_regex = "nanvix"

        # Step 1: merge
        merged_etl = run_xperf_merge(args.etl, xperf_path=args.xperf)

        # Step 2: butterfly
        html = run_xperf_butterfly(
            merged_etl,
            process_filter=proc_regex,
            min_hits=args.min_hits,
            xperf_path=args.xperf,
        )

        # Step 3: parse + report
        stacks_data = parse_butterfly_html(html)
        report = format_stacks_report(stacks_data)
        print(report)

        # JSON output
        if args.json:
            json_report = {
                "etl_file": args.etl,
                "merged_etl": merged_etl,
                "target_processes": processes,
                "stacks": stacks_data,
            }
            with open(args.json, "w") as f:
                json.dump(json_report, f, indent=2, default=str)
            print(f"\nJSON report written to: {args.json}")

        return  # --stacks is a standalone mode

    need_stacks = bool(args.folded)

    # Get xperf dump
    if args.dump_file:
        print(f"[etl] Reading pre-dumped text from {args.dump_file}", file=sys.stderr)
        with open(args.dump_file, "r", encoding="utf-8", errors="replace") as f:
            dump_text = f.read()
    else:
        dump_text = run_xperf_dump(
            args.etl,
            args.xperf,
            symbols=args.symbols,
            stacks=need_stacks,
        )

    print(f"[etl] Parsing events for: {', '.join(processes)}", file=sys.stderr)
    events = parse_events(dump_text, processes)

    print(
        f"[etl] Found: {len(events['cpu_samples'])} CPU samples, "
        f"{len(events['cswitches'])} context switches, "
        f"{len(events['ready_threads'])} ready events",
        file=sys.stderr,
    )

    # Parse comma-separated sections list and validate
    sections = set(s.strip() for s in args.sections.split(",") if s.strip())
    invalid = sections - ALL_SECTIONS
    if invalid:
        inv_str = ", ".join(sorted(invalid))
        all_str = ", ".join(sorted(ALL_SECTIONS))
        parser.error(f"invalid section(s): {inv_str} (choose from {all_str})")
    cpu_analysis = (
        analyze_cpu_samples(events["cpu_samples"]) if "cpu" in sections else {}
    )
    cswitch_analysis = (
        analyze_context_switches(events["cswitches"], events["process_lifetimes"])
        if "cswitch" in sections
        else {}
    )
    sched_analysis = (
        analyze_scheduling(events["ready_threads"]) if "sched" in sections else {}
    )
    timeline = (
        analyze_process_timeline(events["process_lifetimes"], processes)
        if "timeline" in sections
        else []
    )

    # Console report
    report = format_report(
        cpu_analysis, cswitch_analysis, sched_analysis, timeline, sections
    )
    print(report)

    # JSON output
    if args.json:
        json_report = {
            "etl_file": args.etl,
            "target_processes": processes,
        }
        if cpu_analysis:
            json_report["cpu_profile"] = cpu_analysis
        if cswitch_analysis:
            json_report["context_switches"] = cswitch_analysis
        if sched_analysis:
            json_report["scheduling"] = sched_analysis
        if timeline:
            json_report["process_timeline"] = timeline

        with open(args.json, "w") as f:
            json.dump(json_report, f, indent=2)
        print(f"\nJSON report written to: {args.json}")

    # Folded stacks for flamegraph.pl
    if args.folded:
        print("[etl] Extracting full call stacks...", file=sys.stderr)
        folded = parse_folded_stacks(dump_text, processes)
        print(
            f"[etl] Collected {sum(folded.values())} stack samples "
            f"across {len(folded)} unique stacks",
            file=sys.stderr,
        )

        with open(args.folded, "w", encoding="utf-8", newline="\n") as f:
            for stack, count in folded.most_common():
                f.write(f"{stack} {count}\n")
        print(f"Folded stacks written to: {args.folded}")


if __name__ == "__main__":
    main()
