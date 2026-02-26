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
# Standard benchmarks.
./z build -- all RELEASE=yes LOG_LEVEL=panic

# Echo-breakdown benchmark (requires message timestamping).
./z build -- all RELEASE=yes LOG_LEVEL=panic TIMESTAMP_MSG=yes
```

## Available Benchmarks

| Benchmark               | Description                           |
|-------------------------|---------------------------------------|
| `boot-time`             | Start a user VM (no linuxd)           |
| `cold-start`            | Start linuxd + VM + HTTP echo         |
| `cold-start-l2`         | `cold-start` with linuxd in L2 VM     |
| `cold-start-uvm`        | `cold-start` reusing linuxd           |
| `concurrent`            | `cold-start` with many VMs alive      |
| `concurrent-l2`         | `concurrent` with linuxd in L2 VM     |
| `echo-breakdown`        | HTTP echo step-by-step breakdown      |
| `echo-breakdown-l2`     | `echo-breakdown` with linuxd in L2 VM |
| `round-trip-latency`    | Latency vs. echo payload size         |
| `warm-start`            | Fixed-size HTTP echo latency          |
| `warm-start-l2`         | `warm-start` with linuxd in L2 VM     |
| `warm-start-vmm`        | `warm-start` without linuxd           |


## Running Benchmarks

```bash
# Basic usage.
./bin/nanvix-bench.elf -benchmark cold-start
./bin/nanvix-bench.elf -benchmark warm-start
./bin/nanvix-bench.elf -benchmark echo-breakdown

# See all options.
./bin/nanvix-bench.elf -help
```

## Core Pinning (Recommended)

For best performance, pin components to different CPU dies.  Create a JSON config:

```json
{
    "client_core_str": "0-9",
    "linuxd_core_str": "10-14",
    "nanovm_core_str": "15-19"
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
| `echo-c`          | `src/benchmarks/echo-c/`            | C    |
| `echo-cpp`        | `src/benchmarks/echo-cpp/`          | C++  |
| `echo-rust-nostd` | `src/benchmarks/echo-rust-nostd/`   | Rust |
| `echo-wasm-rust`  | `src/benchmarks/echo-wasm-rust/`    | WASM |
| `noop-c`          | `src/benchmarks/noop-c/`            | C    |
| `noop-cpp`        | `src/benchmarks/noop-cpp/`          | C++  |
| `noop-js`         | `src/benchmarks/noop-js/`           | JS   |
| `noop-rust-nostd` | `src/benchmarks/noop-rust-nostd/`   | Rust |
| `noop-wasm-rust`  | `src/benchmarks/noop-wasm-rust/`    | WASM |

## Analyzing Results

Benchmark results can be visualized with the plotting script:

```bash
python3 scripts/plot-performance.py
```

Additional analysis can be done with the automation script:

```bash
python3 scripts/benchmark.py
```
