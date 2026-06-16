#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Flamegraph merge and render.

Takes guest.folded + optional host.folded, applies [GUEST]/[HOST] prefixes,
demangles Rust symbols, and generates the SVG via inferno-flamegraph.
"""

import re
import subprocess
import sys

from pathlib import Path


def _demangle_and_sanitize(folded_path: Path) -> list[str]:
    """Read folded stacks, demangle via rustfilt, and sanitize for SVG."""
    if not folded_path.exists() or folded_path.stat().st_size == 0:
        return []

    result = subprocess.run(
        ["rustfilt"],
        input=folded_path.read_text(encoding="utf-8", errors="replace"),
        capture_output=True,
        text=True,
    )
    lines = (
        result.stdout.splitlines()
        if result.returncode == 0
        else folded_path.read_text(encoding="utf-8", errors="replace").splitlines()
    )

    sanitized = []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        # Remove angle brackets (SVG-incompatible) and hex offsets.
        line = line.replace("<", " ").replace(">", " ")
        # Remove +0x... offsets from symbol names for cleaner display.
        line = re.sub(r"\+0x[0-9a-fA-F]+", "", line)
        sanitized.append(line)
    return sanitized


def _prefix_stacks(lines: list[str], prefix: str) -> list[str]:
    """Add a root frame prefix to each folded stack line."""
    prefixed = []
    for line in lines:
        parts = line.rsplit(" ", 1)
        if len(parts) == 2:
            prefixed.append(f"{prefix};{parts[0]} {parts[1]}")
    return prefixed


def merge_and_render(
    guest_folded: Path,
    host_folded: Path | None,
    svg_path: Path,
    title: str = "Nanvix E2E Flamegraph",
) -> None:
    """Merge guest + host folded stacks and generate SVG."""
    merged_path = svg_path.parent / "merged.folded"

    print("[MERGE] Building unified flamegraph...")

    # Guest stacks -> [GUEST] prefix.
    guest_lines = _demangle_and_sanitize(guest_folded)
    guest_prefixed = _prefix_stacks(guest_lines, "[GUEST]")
    print(f"  [GUEST]: {len(guest_prefixed)} stack entries")

    # Host stacks -> [HOST] prefix (already prefixed by extract_host_stacks).
    host_lines = []
    if host_folded and host_folded.exists() and host_folded.stat().st_size > 0:
        host_lines = [
            line.strip()
            for line in host_folded.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]
        print(f"  [HOST]: {len(host_lines)} stack entries")
    else:
        if host_folded is not None:
            print("  No host stacks (run as admin/root for kernel stacks)")

    # Write merged.
    all_lines = guest_prefixed + host_lines
    merged_path.write_text("\n".join(all_lines) + "\n", encoding="utf-8")

    # Generate SVG.
    if not all_lines:
        print("  WARNING: no stacks to render", file=sys.stderr)
        return

    print("[SVG] Generating flamegraph...")
    result = subprocess.run(
        ["inferno-flamegraph", "--title", title],
        input="\n".join(all_lines),
        capture_output=True,
        text=True,
    )
    if result.returncode == 0:
        svg_path.write_text(result.stdout, encoding="utf-8")
    else:
        print(f"  WARNING: inferno-flamegraph failed: {result.stderr}", file=sys.stderr)
