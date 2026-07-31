---
name: benchmarking
description: Guide for building, running, and analyzing Nanvix performance benchmarks. Use this when asked about benchmark setup, execution, or interpretation.
---

# Benchmarking Nanvix

Use this skill when the user asks about running, creating, or analyzing performance benchmarks in
Nanvix.

## Benchmark Tool

Nanvix includes `nanvix-bench` (`src/utils/nanvix-bench/`) for measuring system performance.
Benchmarks require a release build with panic-level logging.

## Building for Benchmarks

```bash
./z build -- all RELEASE=yes LOG_LEVEL=panic
```

## Available Benchmarks

| Benchmark           | Description                                  |
|---------------------|----------------------------------------------|
| `boot-time`         | Start a user VM without nanvixd              |
| `cold-start`        | Spawn nanvixd + VM + first echo round trip   |
| `cold-start-uvm`    | Start a user VM + first gateway echo         |
| `snapshot-restore`  | Compare snapshot restore with cold boot      |
| `vfs-bench`         | Measure guest VFS operation latency          |
| `warm-start-gateway` | Round-trip latency through the VM gateway    |
| `warm-start-vmm`    | Raw round-trip latency inside the user VM    |
| `warm-start-socket` | TCP echo latency through guest networking    |


## Running Benchmarks

```bash
# Basic usage.
./bin/nanvix-bench.elf -benchmark cold-start
./bin/nanvix-bench.elf -benchmark cold-start-uvm
./bin/nanvix-bench.elf -benchmark warm-start-gateway
./bin/nanvix-bench.elf -benchmark warm-start-vmm
./bin/nanvix-bench.elf -benchmark warm-start-socket

# See all options.
./bin/nanvix-bench.elf -help
```

## Core Pinning (Recommended)

For best performance, pin components to different CPU dies.  Create a JSON config:

```json
{
    "client_core_str": "0-9"
}
```

Then pass it to the benchmark:

```bash
./bin/nanvix-bench.elf \
    -benchmark <benchmark> \
    -hwloc <path_to_config.json>
```

## High-Iteration Runs

For benchmarks with many iterations, increase system limits:

```bash
ulimit -u 65536    # Max user processes.
ulimit -n 65536    # Max open files.
```

## Benchmark Applications

Source code for benchmark programs:

| Benchmark App     | Path                                | Lang |
|-------------------|-------------------------------------|------|
| `echo-rust-nostd` | `src/benchmarks/echo-rust-nostd/`   | Rust |
| `noop-rust-nostd` | `src/benchmarks/noop-rust-nostd/`   | Rust |

## Analyzing Results

Benchmark results can be visualized with the plotting script:

```bash
python3 scripts/plot-performance.py
```

Additional analysis can be done with the automation script:

```bash
python3 scripts/benchmark.py
```

## Benchmarking on Windows

On Windows, the cold-start benchmark spawns a fresh `nanvixd` process per iteration and measures
the time from process spawn to the first echo round trip.

### Building

```powershell
.\z.ps1 build -- all RELEASE=yes LOG_LEVEL=panic

# Explicit native ARM64 equivalent.
.\z.ps1 build -- all TARGET=aarch64 RELEASE=yes LOG_LEVEL=panic
```

### Available Benchmarks

| Benchmark          | Description                                             |
|--------------------|---------------------------------------------------------|
| `boot-time`        | Start a user VM (no nanvixd)                            |
| `cold-start`       | Spawn nanvixd + VM + echo round-trip                    |
| `cold-start-uvm`   | Start a user VM + first gateway echo                    |
| `snapshot-restore` | Measure snapshot restore latency vs boot-time           |
| `warm-start-gateway` | Round-trip latency through the VM gateway              |
| `warm-start-vmm`   | Raw round-trip latency inside the user VM               |

### Running

```powershell
# Using z.ps1.
.\z.ps1 bench -- -benchmark cold-start -iterations 10

# Or directly.
.\bin\nanvix-bench.exe -benchmark cold-start -iterations 10
.\bin\nanvix-bench.exe -benchmark boot-time -iterations 100
.\bin\nanvix-bench.exe -help
```

Use `-help` to list all benchmark options supported by the current build.

`scripts/benchmark.py run` detects the native Windows architecture. It writes X64 and ARM64
results to distinct `_X64.csv` and `_ARM64.csv` histories.
