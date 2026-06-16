#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Platform-specific host stack extraction.

Windows: Extracts kernel stacks from ETL via analyze-etl.py.
Linux:   Extracts kernel stacks from perf.data via perf script + inferno.
"""

import os
import subprocess
import sys

from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent


def extract_host_stacks(
    guest_folded: Path, output_dir: Path, bin_dir: Path | None = None
) -> Path | None:
    """Extract host kernel stacks from the platform-specific trace file.

    Returns the path to host.folded with [HOST] prefixes, or None if
    no host trace was captured.
    """
    if sys.platform == "win32":
        return _extract_etw(guest_folded, output_dir, bin_dir)
    elif sys.platform.startswith("linux"):
        return _extract_perf(guest_folded, output_dir)
    else:
        print(f"  Host stack extraction not supported on {sys.platform}")
        return None


def _extract_etw(
    guest_folded: Path, output_dir: Path, bin_dir: Path | None
) -> Path | None:
    """Extract host stacks from ETL via analyze-etl.py (Windows)."""
    etl_path = Path(f"{guest_folded}.etl")
    if not etl_path.exists():
        print("  No ETL trace (run as admin for host kernel stacks)")
        return None

    analyze_script = SCRIPT_DIR / "analyze-etl.py"
    if not analyze_script.exists():
        print(
            f"  WARNING: analyze-etl.py not found at {analyze_script}", file=sys.stderr
        )
        return None

    # Set up symbol resolution for xperf.
    env = os.environ.copy()
    sym_dir = str(bin_dir) if bin_dir else str(guest_folded.parent)
    if "_NT_SYMBOL_PATH" in env:
        if sym_dir not in env["_NT_SYMBOL_PATH"]:
            env["_NT_SYMBOL_PATH"] = f"{sym_dir};{env['_NT_SYMBOL_PATH']}"
    else:
        env["_NT_SYMBOL_PATH"] = (
            f"{sym_dir};srv*C:\\Symbols*https://msdl.microsoft.com/download/symbols"
        )
    env.setdefault("_NT_SYMCACHE_PATH", "C:\\SymCache")

    print("  Extracting host stacks from ETL (this may take a few minutes)...")
    kernel_folded = output_dir / "kernel.folded"
    result = subprocess.run(
        [
            sys.executable,
            str(analyze_script),
            str(etl_path),
            "--folded",
            str(kernel_folded),
            "--process",
            "nanvixd.exe",
            "--symbols",
        ],
        capture_output=True,
        text=True,
        env=env,
    )
    for line in (result.stdout + result.stderr).splitlines():
        if line.strip():
            print(f"    {line.strip()}")

    if not kernel_folded.exists() or kernel_folded.stat().st_size == 0:
        print("  No host stacks extracted (first run may need symbol download)")
        return None

    # Prefix with [HOST] and return.
    host_folded = output_dir / "host.folded"
    lines = kernel_folded.read_text(encoding="utf-8", errors="replace").splitlines()
    prefixed = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        parts = line.rsplit(" ", 1)
        if len(parts) == 2:
            prefixed.append(f"[HOST];{parts[0]} {parts[1]}")
    host_folded.write_text("\n".join(prefixed) + "\n", encoding="utf-8")
    return host_folded


def _extract_perf(guest_folded: Path, output_dir: Path) -> Path | None:
    """Extract host stacks from perf.data (Linux)."""
    perf_data = Path(f"{guest_folded}.perf.data")
    if not perf_data.exists():
        print("  No perf.data (run as root for host kernel stacks)")
        return None

    print("  Extracting host stacks from perf.data...")

    # Use perf script -F to omit the period field so each event
    # collapses to count=1, matching guest profiler weighting.
    try:
        script_result = subprocess.run(
            [
                "perf",
                "script",
                "-i",
                str(perf_data),
                "-F",
                "comm,pid,tid,time,event,ip,sym,dso",
            ],
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        print("  perf not available -- host stacks skipped")
        return None

    if script_result.returncode != 0:
        print(f"  perf script failed (code {script_result.returncode}):")
        for err_line in script_result.stderr.splitlines()[:5]:
            print(f"    {err_line}")
        return None

    # Collapse stacks via inferno.
    try:
        collapse_result = subprocess.run(
            ["inferno-collapse-perf"],
            input=script_result.stdout,
            capture_output=True,
            text=True,
        )
    except FileNotFoundError:
        print("  inferno-collapse-perf not available -- host stacks skipped")
        return None

    if collapse_result.returncode != 0:
        print(f"  inferno-collapse-perf failed (code {collapse_result.returncode}):")
        for err_line in collapse_result.stderr.splitlines()[:5]:
            print(f"    {err_line}")
        return None

    # Filter to nanvixd/uservm stacks and prefix with [HOST].
    host_folded = output_dir / "host.folded"
    lines = []
    for line in collapse_result.stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        if "nanvixd" in line.lower() or "uservm" in line.lower():
            lines.append(f"[HOST];{line}")
    if not lines:
        print("  No host stacks for nanvixd (perf may need CAP_SYS_ADMIN)")
        return None

    host_folded.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return host_folded
