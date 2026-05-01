#!/usr/bin/env python3
# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Nanvix E2E Flamegraph Generator

Produces unified [GUEST]/[HOST] flamegraphs spanning guest kernel + user
application + host VMM + host OS kernel.

Usage:
    # Full E2E (guest + host kernel stacks, requires admin/root):
    python scripts/bench/flamegraph.py full --guest-elf bin/python.elf

    # Guest-only flamegraph:
    python scripts/bench/flamegraph.py guest --guest-elf bin/python.elf

    # With pre-built ramfs:
    python scripts/bench/flamegraph.py full --ramfs my.img --guest-elf bin/app.elf
"""

import argparse
import os
import shutil
import subprocess
import sys
import textwrap

from pathlib import Path

# Resolve repo root from script location: scripts/bench/flamegraph.py -> ../..
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent


def find_tool(name: str) -> str | None:
    """Find an executable on PATH or in common locations."""
    return shutil.which(name)


def check_prerequisites(full_mode: bool) -> None:
    """Fail early if required tools are missing."""
    missing = []
    for tool in ["rustfilt", "inferno-flamegraph"]:
        if not find_tool(tool):
            missing.append(tool)
    if full_mode and sys.platform == "win32":
        if not find_tool("wpr"):
            missing.append("wpr (Windows Performance Toolkit)")
    if missing:
        print(f"ERROR: Missing tools: {', '.join(missing)}", file=sys.stderr)
        print("  Install with: cargo install inferno rustfilt", file=sys.stderr)
        sys.exit(1)


def build_ramfs(
    mkramfs: Path, guest_elf: Path, script_content: str, output_dir: Path
) -> Path:
    """Build a ramfs image containing the guest ELF and a Python script."""
    ramfs_dir = output_dir / "ramfs-content"
    ramfs_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(guest_elf, ramfs_dir / guest_elf.name)
    (ramfs_dir / "script.py").write_text(script_content, encoding="utf-8")

    ramfs_img = output_dir / "test.img"
    subprocess.run(
        [str(mkramfs), "-o", str(ramfs_img), str(ramfs_dir)],
        check=True,
        capture_output=True,
    )
    return ramfs_img


def run_nanvixd(
    nanvixd: Path,
    bin_dir: Path,
    ramfs: Path,
    guest_elf: Path,
    guest_folded: Path,
    kernel_symbols: str,
    user_symbols: str,
    guest_arg: str = "",
    timeout: int = 600,
) -> tuple[str, str]:
    """Run nanvixd with profiler env vars and return (stdout, stderr)."""
    env = os.environ.copy()
    env["NANVIX_GUEST_PROFILE_PATH"] = str(guest_folded)
    if kernel_symbols:
        env["NANVIX_KERNEL_SYMBOLS"] = kernel_symbols
    if user_symbols:
        env["NANVIX_USER_SYMBOLS"] = user_symbols

    cmd = [
        str(nanvixd),
        "-bin-dir",
        str(bin_dir),
        "-ramfs",
        str(ramfs),
        "--",
        str(guest_elf),
    ]
    if guest_arg:
        cmd.extend(guest_arg.split())
    result = subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        timeout=timeout,
        env=env,
    )
    if result.returncode != 0:
        print(
            f"  WARNING: nanvixd exited with code {result.returncode}", file=sys.stderr
        )
    return result.stdout, result.stderr


def default_script() -> str:
    """Default Python workload for profiling."""
    return textwrap.dedent("""\
        import math
        for i in range(200):
            result = sum(math.factorial(j) for j in range(200))
            print(f'iter {i}: {len(str(result))} digits')
        print('Done')
    """)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Nanvix E2E Flamegraph Generator",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "mode",
        choices=["guest", "full"],
        help="'guest' for guest-only, 'full' for guest + host kernel stacks",
    )
    parser.add_argument("--nanvix-dir", type=Path, default=REPO_ROOT)
    parser.add_argument("--guest-elf", type=Path, required=True)
    parser.add_argument("--kernel-symbols", default="")
    parser.add_argument("--user-symbols", default="")
    parser.add_argument(
        "--script", default="", help="Inline Python script content for guest workload"
    )
    parser.add_argument(
        "--guest-arg",
        default="-B /script.py",
        help="Arguments passed to the guest ELF (default: '-B /script.py')",
    )
    parser.add_argument("--output-dir", type=Path, default=None)
    parser.add_argument("--ramfs", type=Path, default=None)
    parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()

    nanvix_dir = args.nanvix_dir.resolve()
    bin_dir = nanvix_dir / "bin"
    ext = ".exe" if sys.platform == "win32" else ".elf"
    nanvixd = bin_dir / f"nanvixd{ext}"
    mkramfs = bin_dir / f"mkramfs{ext}"

    if args.output_dir is None:
        args.output_dir = nanvix_dir.parent / "profiling-output"
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    if not args.kernel_symbols:
        args.kernel_symbols = str(bin_dir / "kernel.elf")

    check_prerequisites(args.mode == "full")

    for tool in [nanvixd, mkramfs]:
        if not tool.exists():
            print(
                f"ERROR: {tool} not found. Run 'z build --profile --release' first.",
                file=sys.stderr,
            )
            sys.exit(1)
    if not args.guest_elf.exists():
        print(f"ERROR: Guest ELF not found: {args.guest_elf}", file=sys.stderr)
        sys.exit(1)

    # -- Banner --
    print("=" * 65)
    title = (
        "Nanvix Guest Flamegraph"
        if args.mode == "guest"
        else "Nanvix Full E2E Flamegraph"
    )
    print(f"  {title}")
    print("=" * 65)
    print()

    # -- Build ramfs --
    cleanup_ramfs = False
    ramfs = args.ramfs
    if ramfs is None or not ramfs.exists():
        print("[SETUP] Building ramfs...")
        script_content = args.script if args.script else default_script()
        ramfs = build_ramfs(mkramfs, args.guest_elf, script_content, output_dir)
        cleanup_ramfs = True
        print(f"  Ramfs: {ramfs}")

    # -- Run nanvixd --
    guest_folded = output_dir / "guest.folded"
    print("[RUN] Running nanvixd with profiling...")
    try:
        stdout, stderr = run_nanvixd(
            nanvixd,
            bin_dir,
            ramfs,
            args.guest_elf,
            guest_folded,
            args.kernel_symbols,
            args.user_symbols,
            args.guest_arg,
            args.timeout,
        )
    except subprocess.TimeoutExpired:
        print("ERROR: nanvixd timed out", file=sys.stderr)
        sys.exit(1)

    # Show output summary.
    stdout_lines = [line for line in stdout.splitlines() if line.strip()]
    if len(stdout_lines) > 5:
        print(f"  ... ({len(stdout_lines)} lines of output)")
        for line in stdout_lines[-3:]:
            print(f"  {line}")
    else:
        for line in stdout_lines:
            print(f"  {line}")

    # Show profiler lines from stderr.
    (output_dir / "nanvixd-stderr.txt").write_text(stderr, encoding="utf-8")
    for line in stderr.splitlines():
        if any(
            k in line
            for k in ("GUEST_PROFILE", "PROFILER", "ETW_SESSION", "PERF_SESSION")
        ):
            print(f"  {line.strip()}")

    if not guest_folded.exists():
        print("WARNING: no guest folded stacks produced", file=sys.stderr)

    # -- Merge and generate SVG --
    # Add script directory to path for sibling module imports.
    if str(SCRIPT_DIR) not in sys.path:
        sys.path.insert(0, str(SCRIPT_DIR))
    from flamegraph_merge import merge_and_render
    from flamegraph_host import extract_host_stacks

    host_folded = None
    if args.mode == "full":
        host_folded = extract_host_stacks(guest_folded, output_dir, bin_dir)

    svg_path = output_dir / "flamegraph.svg"
    merge_and_render(guest_folded, host_folded, svg_path)

    # -- Summary --
    print()
    print("=" * 65)
    print("  Results")
    print("=" * 65)
    print()
    for f in sorted(output_dir.iterdir()):
        if f.suffix in (".svg", ".folded", ".txt", ".etl", ".data"):
            size = f.stat().st_size
            unit = "MB" if size > 1_000_000 else "KB"
            val = size / 1_000_000 if size > 1_000_000 else size / 1_000
            print(f"  {f.name}: {val:.1f} {unit}")
    if svg_path.exists():
        print(f"\n  Flamegraph: {svg_path}")

    # -- Cleanup --
    if cleanup_ramfs:
        shutil.rmtree(output_dir / "ramfs-content", ignore_errors=True)
        (output_dir / "test.img").unlink(missing_ok=True)


if __name__ == "__main__":
    main()
