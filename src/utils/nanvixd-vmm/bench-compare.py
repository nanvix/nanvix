#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Side-by-side performance comparison: Nanvix on OpenVMM vs its own uservm VMM.

This runs the **native** Nanvix `nanvix-bench` standalone benchmarks twice using
the *same* measurement code, changing only the VMM front-end being driven:

  - uservm  : the native `bin/nanvixd.elf` (Nanvix's own KVM/WHP VMM), and
  - openvmm : the OpenVMM-based `nanvixd-vmm` (this port),

selected via the `NANVIX_BENCH_NANVIXD` environment variable the harness honors.
Because one driver measures both, the comparison is apples-to-apples.

Supported benchmarks (the standalone-applicable set): `cold-start`, `vfs-bench`.

Prerequisites (release builds, for a fair comparison):
  ./z build --release LOG_LEVEL=panic                 # guests + bin/nanvixd.elf
  src/utils/nanvixd-vmm/build.sh --release             # target/release/nanvixd-vmm
  RELEASE=yes LOG_LEVEL=panic MEMORY_SIZE_BYTES=$((128*1048576)) \\
      cargo build -p nanvix-bench --no-default-features --features standalone,microvm

Usage:
    bench-compare.py [--benchmark cold-start|vfs-bench|all] [--iterations N]
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import time
import select
from pathlib import Path


def nanvix_root() -> Path:
    return Path(__file__).resolve().parents[3]


def percentiles(latencies_us: list[float]) -> dict[str, int]:
    s = sorted(latencies_us)
    n = len(s)

    def pick(p: float) -> int:
        return int(s[min(int(n * p), n - 1)])

    return {
        "min": int(s[0]),
        "p50": pick(0.50),
        "p95": pick(0.95),
        "p99": pick(0.99),
        "mean": int(sum(s) / n),
    }


def run_warm_start(
    vmm: Path | None, iterations: int, warmup: int, payload_size: int, timeout: int
) -> dict[str, int]:
    """Measures the warm round-trip latency of one host->guest->host echo cycle.

    Boots the echo guest once, then repeatedly writes `payload_size` bytes to the
    VMM's stdin and reads them back from its stdout, timing each round trip. This
    is the subprocess (black-box) analogue of the native `warm-start-vmm`: it
    measures the communication latency *in and out of the VMM* through its IKC
    stdio bridge. `vmm=None` uses the native uservm `bin/nanvixd.elf`.
    """
    root = nanvix_root()
    binary = vmm if vmm is not None else (root / "bin" / "nanvixd.elf")
    program = "./bin/echo-rust-nostd.initrd"
    payload = bytes((i % 251) for i in range(payload_size))

    proc = subprocess.Popen(
        [str(binary), "-bin-dir", "./bin", "--", program],
        cwd=str(root),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        bufsize=0,
    )
    out_fd = proc.stdout.fileno()

    def read_exact(n: int, deadline: float) -> bytes:
        buf = b""
        while len(buf) < n:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("echo round-trip timed out")
            r, _, _ = select.select([out_fd], [], [], remaining)
            if not r:
                continue
            chunk = os.read(out_fd, n - len(buf))
            if not chunk:
                raise EOFError("guest closed stdout")
            buf += chunk
        return buf

    def one_round_trip() -> float:
        t0 = time.monotonic()
        proc.stdin.write(payload)
        proc.stdin.flush()
        read_exact(len(payload), t0 + timeout)
        return (time.monotonic() - t0) * 1e6  # microseconds

    try:
        for _ in range(warmup):
            one_round_trip()
        latencies = [one_round_trip() for _ in range(iterations)]
    finally:
        try:
            proc.stdin.close()
        except Exception:
            pass
        try:
            proc.wait(timeout=5)
        except Exception:
            proc.kill()

    return percentiles(latencies)


def compare_warm_start(
    iters: int, openvmm: Path, timeout: int, payload: int, warmup: int
) -> None:
    print(
        f"\n### warm-start  ({iters} iterations, {payload}-byte payload, "
        f"{warmup} warmup) — round-trip latency in microseconds\n"
    )
    uvm = run_warm_start(None, iters, warmup, payload, timeout)
    ovm = run_warm_start(openvmm, iters, warmup, payload, timeout)
    print(f"{'metric':<10} {'uservm':>10} {'openvmm':>10} {'uvm/ovm':>9}")
    print("-" * 42)
    for k in ("min", "p50", "p95", "p99", "mean"):
        print(f"{k:<10} {uvm[k]:>10} {ovm[k]:>10} {ratio(uvm[k], ovm[k]):>9}")


def nanvix_bench_binary() -> Path:
    """Prefers the release nanvix-bench (fair white-box uservm), else debug."""
    root = nanvix_root()
    rel = root / "target" / "release" / "nanvix-bench"
    dbg = root / "target" / "debug" / "nanvix-bench"
    if rel.exists():
        return rel
    if dbg.exists():
        return dbg
    sys.exit("error: nanvix-bench not found (build it; see header).")


def run_bench(bench: str, iterations: int, nanvixd: Path | None, timeout: int) -> str:
    """Runs nanvix-bench once; returns its stdout. `nanvixd=None` uses uservm."""
    root = nanvix_root()
    binary = nanvix_bench_binary()
    env = dict(os.environ)
    if nanvixd is not None:
        env["NANVIX_BENCH_NANVIXD"] = str(nanvixd)
    else:
        env.pop("NANVIX_BENCH_NANVIXD", None)
    proc = subprocess.run(
        [str(binary), "-benchmark", bench, "-iterations", str(iterations)],
        cwd=str(root),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=timeout,
    )
    return proc.stdout.decode(errors="replace")


def parse_latency(text: str) -> dict[str, int]:
    """Parses any of first/min/p50/p95/p99/mean latency lines (`<key>: N us`)."""
    out: dict[str, int] = {}
    for key, label in (
        ("First req", "first"),
        ("min", "min"),
        ("p50", "p50"),
        ("p95", "p95"),
        ("p99", "p99"),
        ("mean", "mean"),
    ):
        m = re.search(rf"(?m)^\s*{re.escape(key)}:\s*(\d+)\s*us", text)
        if m:
            out[label] = int(m.group(1))
    return out


def run_openvmm_warmstart_inproc(
    warmstart_bin: Path, iters: int, warmup: int, payload: int, timeout: int
) -> dict[str, int]:
    """Runs the in-process (white-box) OpenVMM warm-start binary."""
    proc = subprocess.run(
        [
            str(warmstart_bin),
            "-bin-dir",
            "./bin",
            "-iterations",
            str(iters),
            "-warmup",
            str(warmup),
            "-payload",
            str(payload),
        ],
        cwd=str(nanvix_root()),
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=timeout,
    )
    return parse_latency(proc.stdout.decode(errors="replace"))


def compare_warm_start_vmm(
    iters: int, openvmm: Path, timeout: int, payload: int, warmup: int
) -> None:
    """White-box: raw in/out-VMM IKC round trip, in-process (no OS pipe)."""
    print(
        f"\n### warm-start-vmm  ({iters} iterations) — WHITE-BOX in-process IKC "
        f"round-trip latency (us)\n"
    )
    warmstart_bin = openvmm.with_name("nanvixd-vmm-warmstart")
    if not warmstart_bin.exists():
        sys.exit(f"error: {warmstart_bin} not found; run build.sh --release.")
    # uservm: native in-process `warm-start-vmm` (links uservm; needs release nanvix-bench).
    uvm = parse_latency(run_bench("warm-start-vmm", iters, None, timeout))
    # openvmm: in-process ChannelGuestIo round-trip.
    ovm = run_openvmm_warmstart_inproc(warmstart_bin, iters, warmup, payload, timeout)
    print(f"{'metric':<10} {'uservm':>10} {'openvmm':>10} {'uvm/ovm':>9}")
    print("-" * 42)
    for k in ("p50", "p95", "p99"):
        if k in uvm and k in ovm:
            print(f"{k:<10} {uvm[k]:>10} {ovm[k]:>10} {ratio(uvm[k], ovm[k]):>9}")


def parse_cold_start(text: str) -> dict[str, int]:
    return parse_latency(text)


def parse_vfs(text: str) -> dict[str, dict[str, int]]:
    """Returns {"<section>/<op>": {p50,p95,p99}}."""
    rows: dict[str, dict[str, int]] = {}
    section = None
    for line in text.splitlines():
        if line.startswith("Writable mount"):
            section = "writable"
            continue
        if line.startswith("Read-only mount"):
            section = "readonly"
            continue
        m = re.match(r"^(\S[\S+]*)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s*$", line)
        if m and section:
            name, _samples, p50, p95, p99 = m.groups()
            rows[f"{section}/{name}"] = {
                "p50": int(p50),
                "p95": int(p95),
                "p99": int(p99),
            }
    return rows


def ratio(a: int, b: int) -> str:
    """uservm/openvmm ratio (>1 means OpenVMM faster)."""
    if b == 0:
        return "n/a"
    return f"{a / b:.2f}x"


def compare_cold_start(iters: int, openvmm: Path, timeout: int) -> None:
    print(f"\n### cold-start  ({iters} iterations) — latency in microseconds\n")
    uvm = parse_cold_start(run_bench("cold-start", iters, None, timeout))
    ovm = parse_cold_start(run_bench("cold-start", iters, openvmm, timeout))
    print(f"{'metric':<10} {'uservm':>10} {'openvmm':>10} {'uvm/ovm':>9}")
    print("-" * 42)
    for k in ("first", "p50", "p95", "p99"):
        if k in uvm and k in ovm:
            print(f"{k:<10} {uvm[k]:>10} {ovm[k]:>10} {ratio(uvm[k], ovm[k]):>9}")


def compare_vfs(iters: int, openvmm: Path, timeout: int) -> None:
    print(f"\n### vfs-bench  ({iters} iterations) — per-op latency (us), p50\n")
    uvm = parse_vfs(run_bench("vfs-bench", iters, None, timeout))
    ovm = parse_vfs(run_bench("vfs-bench", iters, openvmm, timeout))
    print(f"{'section/op':<30} {'uservm':>8} {'openvmm':>8} {'uvm/ovm':>9}")
    print("-" * 58)
    for key in uvm:
        if key in ovm:
            a, b = uvm[key]["p50"], ovm[key]["p50"]
            print(f"{key:<30} {a:>8} {b:>8} {ratio(a, b):>9}")


def main() -> int:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "--benchmark",
        choices=["cold-start", "vfs-bench", "warm-start", "warm-start-vmm", "all"],
        default="all",
    )
    ap.add_argument(
        "--iterations",
        type=int,
        default=0,
        help="override iterations (default: 100 cold-start, 1000 others)",
    )
    ap.add_argument(
        "--payload",
        type=int,
        default=32,
        help="warm-start round-trip payload size in bytes (default: 32)",
    )
    ap.add_argument(
        "--warmup",
        type=int,
        default=50,
        help="warm-start untimed warmup round trips (default: 50)",
    )
    ap.add_argument(
        "--openvmm",
        type=Path,
        default=None,
        help="path to nanvixd-vmm (default: target/release/nanvixd-vmm)",
    )
    ap.add_argument("--timeout", type=int, default=600)
    args = ap.parse_args()

    openvmm = (
        args.openvmm or (nanvix_root() / "target" / "release" / "nanvixd-vmm")
    ).resolve()
    if not openvmm.exists():
        sys.exit(
            f"error: OpenVMM binary not found at {openvmm}; run build.sh --release."
        )

    print(f"uservm  = {nanvix_root() / 'bin' / 'nanvixd.elf'}")
    print(f"openvmm = {openvmm}")

    if args.benchmark in ("cold-start", "all"):
        compare_cold_start(args.iterations or 100, openvmm, args.timeout)
    if args.benchmark in ("warm-start", "all"):
        compare_warm_start(
            args.iterations or 1000,
            openvmm,
            min(args.timeout, 30),
            args.payload,
            args.warmup,
        )
    if args.benchmark in ("warm-start-vmm", "all"):
        compare_warm_start_vmm(
            args.iterations or 1000, openvmm, args.timeout, args.payload, args.warmup
        )
    if args.benchmark in ("vfs-bench", "all"):
        compare_vfs(args.iterations or 1000, openvmm, args.timeout)
    return 0


if __name__ == "__main__":
    sys.exit(main())
