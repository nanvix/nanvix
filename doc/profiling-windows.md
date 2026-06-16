# Profiling Nanvix (Windows)

> **Prerequisite:** You must be able to build and run Nanvix before profiling.
> See [build](build-windows.md) and [run](run-windows.md) for instructions.

This document describes the profiling infrastructure for reproducible Windows/WHP
performance debugging. It covers system tuning, benchmark execution with ETW tracing,
and post-processing tools for phase analysis and flamegraph generation.

## Table of Contents

- [Table of Contents](#table-of-contents)
- [1. Prerequisites](#1-prerequisites)
- [2. One-Time Setup](#2-one-time-setup)
  - [Perl (for flamegraphs)](#perl-for-flamegraphs)
  - [FlameGraph Tools](#flamegraph-tools)
  - [Symbol Resolution](#symbol-resolution)
  - [Windows Performance Toolkit](#windows-performance-toolkit)
- [3. Quick Start](#3-quick-start)
  - [Phase Timing Only](#phase-timing-only)
  - [With WPR Trace (Deep Kernel Investigation)](#with-wpr-trace-deep-kernel-investigation)
  - [Flamegraph Generation](#flamegraph-generation)
  - [Kernel Stack Analysis](#kernel-stack-analysis)
- [4. Performance Debugging Workflow](#4-performance-debugging-workflow)
  - [Step 1. Establish a Baseline](#step-1-establish-a-baseline)
  - [Step 2. Identify Bottlenecks](#step-2-identify-bottlenecks)
  - [Step 3. Implement Optimization, Then Compare](#step-3-implement-optimization-then-compare)
  - [Step 4. Drill Deeper (If Needed)](#step-4-drill-deeper-if-needed)
  - [Step 5. Restore System](#step-5-restore-system)
- [5. Script Reference](#5-script-reference)
  - [`bench-setup.ps1`](#bench-setupps1)
  - [`bench-teardown.ps1`](#bench-teardownps1)
  - [`bench-run.ps1`](#bench-runps1)
  - [`analyze-results.py`](#analyze-resultspy)
  - [`analyze-etl.py`](#analyze-etlpy)
  - [`wpr-profile.wprp`](#wpr-profilewprp)
- [6. Understanding the Output](#6-understanding-the-output)
  - [Phase Breakdown](#phase-breakdown)
  - [Distribution Analysis (from PERF\_TIMINGS)](#distribution-analysis-from-perf_timings)
  - [Bottleneck Analysis](#bottleneck-analysis)
  - [PERF\_TIMINGS Data Format](#perf_timings-data-format)
- [7. Guest Flamegraph Profiling](#7-guest-flamegraph-profiling)

---

## 1. Prerequisites

- **Windows 10/11 or Windows Server 2019+** with Hyper-V and WHP enabled
- **Administrator privileges** (for power plan, timer resolution, Realtime priority)
- **Python 3.10+** (stdlib only, no pip packages required)
- **Rust toolchain** matching the `rust-toolchain` file
- **WPR** (ships with Windows) -- only needed for ETW trace capture
- **Perl** -- required for flamegraph generation (see below)

## 2. One-Time Setup

### Perl (for flamegraphs)

Perl is bundled with **Git for Windows**. Verify it works:

```powershell
# Git-bundled Perl (most common).
& "C:\Program Files\Git\usr\bin\perl.exe" --version

# If perl is in PATH (e.g., Strawberry Perl installed separately).
perl --version
```

If Perl is not available, install [Git for Windows](https://gitforwindows.org/)
or [Strawberry Perl](https://strawberryperl.com/).

### FlameGraph Tools

Clone Brendan Gregg's FlameGraph repository into the `tools/` directory
(already in `.gitignore`):

```powershell
cd <nanvix-repo-root>
git clone --depth 1 https://github.com/brendangregg/FlameGraph.git tools/FlameGraph
```

Verify it works:

```powershell
& "C:\Program Files\Git\usr\bin\perl.exe" tools\FlameGraph\flamegraph.pl --help
```

### Symbol Resolution

Set up the Microsoft symbol server for resolving kernel function names:

```powershell
# Add to your PowerShell profile ($PROFILE) for persistence.
$env:_NT_SYMBOL_PATH = "srv*C:\Symbols*https://msdl.microsoft.com/download/symbols;<nanvix-repo>\target\release"
$env:_NT_SYMCACHE_PATH = "C:\SymCache"
```

The first run with `--symbols` will download and cache symbols (~1-2 min).
Subsequent runs reuse the cache.

### Windows Performance Toolkit

xperf is required for ETL trace analysis. It ships with the
[Windows SDK](https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/):

```powershell
# Verify xperf is available (default install path).
& "C:\Program Files (x86)\Windows Kits\10\Windows Performance Toolkit\xperf.exe" -help providers 2>&1 | Select-Object -First 1
```

If not installed, download the Windows SDK and select only
"Windows Performance Toolkit" during installation.

## 3. Quick Start

### Phase Timing Only

```powershell
# 1. Build with profiling enabled (--profile implies PROFILER=yes).
.\z.ps1 build --profile --release -- LOG_LEVEL=panic

# 2. Open an elevated PowerShell, then tune the system for benchmarking.
.\scripts\bench\bench-setup.ps1 -SaveState

# 3. Run the benchmark (use actual filenames from bench-run output).
.\scripts\bench\bench-run.ps1 -Benchmark cold-start -Iterations 50 -OutputDir .\results

# 4. Analyze results.
python scripts\bench\analyze-results.py report `
    --stdout .\results\cold-start-YYYYMMDD-HHMMSS-stdout.txt `
    --stderr .\results\cold-start-YYYYMMDD-HHMMSS-stderr.txt `
    --json .\results\report.json

# 5. Restore system to normal.
.\scripts\bench\bench-teardown.ps1
```

> **Note:** The `--profile` flag is required for PERF_TIMINGS distribution analysis.
> Without it, only the phase breakdown table (p50/p95/p99) from stdout is available.

### With WPR Trace (Deep Kernel Investigation)

```powershell
# Capture ETL trace alongside the benchmark.
.\scripts\bench\bench-run.ps1 -Benchmark cold-start -Iterations 10 -WPR -OutputDir .\traces

# Analyze the ETL trace (CPU profile, context switches, scheduling).
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl

# With symbol resolution (resolves kernel function names).
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --symbols

# JSON output.
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --json .\traces\etl-report.json

# Open in WPA for interactive drill-down.
wpa .\traces\cold-start-YYYYMMDD-HHMMSS.etl
```

### Flamegraph Generation

Interactive flamegraphs visualize CPU call stacks as zoomable SVGs using
[Brendan Gregg's FlameGraph tools](https://github.com/brendangregg/FlameGraph).

```powershell
# 1. Generate folded stacks from ETL trace (--folded implies --symbols).
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --folded .\traces\stacks.folded

# 2. Generate interactive SVG flamegraph.
#    Using Git-bundled Perl:
& "C:\Program Files\Git\usr\bin\perl.exe" tools\FlameGraph\flamegraph.pl `
    --title "Nanvix Cold-Start CPU Profile" `
    --countname samples `
    .\traces\stacks.folded > .\traces\flamegraph.svg

#    Or if perl is in PATH:
perl tools\FlameGraph\flamegraph.pl .\traces\stacks.folded > .\traces\flamegraph.svg

# 3. Open in browser (click bars to zoom, Ctrl+F to search).
start .\traces\flamegraph.svg
```

**Tips:**

- Filter to a single process: `--process nanvixd.exe`
- Wider SVGs: add `--width 1800` to flamegraph.pl
- Reverse (icicle) graph: add `--inverted` to flamegraph.pl
- The folded stacks file is plain text -- you can grep/filter it before
  passing to flamegraph.pl (e.g., `Select-String "ntoskrnl"` for kernel-only)

### Kernel Stack Analysis

The `--stacks` flag provides a structured breakdown of where CPU time is spent
at the module and function level. This is the most direct way to identify
kernel hotspots (e.g., WHP hypercall overhead, EPT fault handling, memory
management).

```powershell
# Run kernel stack analysis on a WPR trace.
# This merges the ETL (for symbol resolution), runs xperf butterfly analysis,
# and produces a module/function breakdown with WHP call chain details.
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --stacks

# Save structured output as JSON for further processing.
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --stacks --json stacks.json

# Use higher minimum hit threshold to reduce noise in large traces.
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --stacks --min-hits 10

# Focus analysis on specific processes.
python scripts\bench\analyze-etl.py .\traces\cold-start-YYYYMMDD-HHMMSS.etl --stacks --process nanvixd.exe
```

**Output sections:**

- **Modules by Exclusive Hits** -- which DLLs/drivers consume CPU (expect
  `ntkrnlmp.exe` to dominate for WHP workloads).
- **Top Functions by Exclusive Hits** -- the specific kernel functions where
  CPU time is spent (e.g., `KeExpandKernelStackAndCalloutInternal` for
  WHP hypercalls, `MiInitializePfn` for page faults).
- **Top Functions by Inclusive Hits** -- call chain attribution showing which
  high-level operations lead to CPU consumption.
- **WHP / Hypervisor Functions** -- filtered view of `WHv*`, `vid.*`, and
  `WinHvR.*` functions showing the hypervisor API call chain
  (e.g., `WHvRunVirtualProcessor` -> `Memory::ResolveFault` for EPT faults).

**Prerequisites:**

- `_NT_SYMBOL_PATH` must be set (see [Symbol Resolution](#symbol-resolution)).
- `xperf.exe` must be installed (see [Windows Performance Toolkit](#windows-performance-toolkit)).
- The trace must include `SampledProfile` and `CSwitch` stack walks (the
  default `wpr-profile.wprp` includes both).

## 4. Performance Debugging Workflow

A typical optimization workflow using these tools:

### Step 1. Establish a Baseline

```powershell
# Build, setup, and run baseline (elevated shell required).
.\z.ps1 build --profile --release -- LOG_LEVEL=panic
.\scripts\bench\bench-setup.ps1 -SaveState
.\scripts\bench\bench-run.ps1 -Iterations 50 -WPR -OutputDir .\baseline
```

### Step 2. Identify Bottlenecks

```powershell
# Phase timing analysis (which phases are slowest?).
python scripts\bench\analyze-results.py report `
    --stdout .\baseline\cold-start-*-stdout.txt `
    --stderr .\baseline\cold-start-*-stderr.txt

# CPU profile (where is kernel/user time spent?).
python scripts\bench\analyze-etl.py .\baseline\cold-start-*.etl --symbols

# Flamegraph (which call stacks dominate?).
python scripts\bench\analyze-etl.py .\baseline\cold-start-*.etl --folded .\baseline\stacks.folded
& "C:\Program Files\Git\usr\bin\perl.exe" tools\FlameGraph\flamegraph.pl `
    --title "Baseline" .\baseline\stacks.folded > .\baseline\flamegraph.svg
start .\baseline\flamegraph.svg
```

### Step 3. Implement Optimization, Then Compare

```powershell
# Build and run the optimized version.
.\z.ps1 build --profile --release -- LOG_LEVEL=panic
.\scripts\bench\bench-run.ps1 -Iterations 50 -WPR -OutputDir .\optimized

# A/B comparison (delta per phase + outlier detection).
python scripts\bench\analyze-results.py compare `
    --before .\baseline\cold-start-*-stderr.txt `
    --after  .\optimized\cold-start-*-stderr.txt
```

### Step 4. Drill Deeper (If Needed)

```powershell
# Per-process flamegraph (only nanvixd kernel activity).
python scripts\bench\analyze-etl.py .\optimized\cold-start-*.etl `
    --folded .\optimized\nanvixd.folded --process nanvixd.exe
& "C:\Program Files\Git\usr\bin\perl.exe" tools\FlameGraph\flamegraph.pl `
    .\optimized\nanvixd.folded > .\optimized\nanvixd-flame.svg

# Filter folded stacks to specific subsystems.
Select-String "ntoskrnl" .\optimized\stacks.folded > .\optimized\kernel-only.folded
Select-String "winhvr\|hvax64" .\optimized\stacks.folded > .\optimized\hypervisor-only.folded

# Export per-iteration CSV for spreadsheet analysis.
python scripts\bench\analyze-results.py report `
    --stdout .\optimized\cold-start-*-stdout.txt `
    --stderr .\optimized\cold-start-*-stderr.txt `
    --perf-csv .\optimized\iterations.csv
```

### Step 5. Restore System

```powershell
.\scripts\bench\bench-teardown.ps1
```

## 5. Script Reference

All scripts are located in `scripts/bench/`.

### `bench-setup.ps1`

Tunes the system for low-noise benchmarking. Must be run as administrator.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `-SaveState` | Switch | Off | Save current settings for later restore |
| `-RepoPath` | String | Auto-detect | Path to nanvix repository |
| `-Quiet` | Switch | Off | Suppress info output |

**What it does:**

- Switches to High Performance power plan
- Locks CPU frequency at 100% (PROCTHROTTLEMIN=100, PROCTHROTTLEMAX=100)
- Sets Energy Performance Preference to 0 (max performance)
- Adds Windows Defender exclusion for the repo
- Stops background services: WSearch, DiagTrack, SysMain, TabletInputService

### `bench-teardown.ps1`

Restores system to pre-benchmark state. Requires `.bench-state.json`
created by `bench-setup.ps1 -SaveState`.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `-KeepDefenderExclusion` | Switch | Off | Keep Defender exclusion |

### `bench-run.ps1`

Runs a benchmark with CPU pinning and Realtime priority. Must be run as administrator.

| Parameter | Type | Default | Description |
| --- | --- | --- | --- |
| `-Benchmark` | String | `cold-start` | Benchmark name |
| `-Iterations` | Int | `50` | Number of iterations |
| `-AffinityMask` | UInt64 | `0xF00` | CPU affinity mask |
| `-OutputDir` | String | `.` | Output directory |
| `-WPR` | Switch | Off | Enable WPR trace capture |
| `-ExtraArgs` | String[] | `@()` | Extra args for nanvix-bench |

**Outputs** (in OutputDir):

- `{benchmark}-{timestamp}-stdout.txt` -- benchmark text output
- `{benchmark}-{timestamp}-stderr.txt` -- PERF_TIMINGS data
- `{benchmark}-{timestamp}.etl` -- WPR trace (only with `-WPR`)

> **Note:** Does NOT call setup/teardown. Caller is responsible for system tuning.

### `analyze-results.py`

Analyzes benchmark results and detects regressions.

**Report command** -- analyze a single benchmark run:

```text
python analyze-results.py report --stdout FILE --stderr FILE [--baseline CSV] [--json PATH] [--perf-csv PATH]
```

| Argument | Required | Description |
| --- | --- | --- |
| `--stdout` | Yes | Path to benchmark stdout file |
| `--stderr` | No | Path to stderr file with PERF_TIMINGS |
| `--baseline` | No | Baseline CSV for regression detection |
| `--json` | No | Write JSON report to this path |
| `--perf-csv` | No | Write per-iteration data to CSV |

**Compare command** -- A/B comparison of two benchmark runs:

```text
python analyze-results.py compare --before STDERR_A --after STDERR_B [--json PATH]
```

| Argument | Required | Description |
| --- | --- | --- |
| `--before` | Yes | Stderr from the baseline/before run |
| `--after` | Yes | Stderr from the optimized/after run |
| `--json` | No | Write JSON comparison to this path |

The compare command shows per-phase delta (absolute and percentage), variability
changes (CV%), and outlier detection. Useful for validating optimizations.

**Regression thresholds:**

- `[WARN]` Warning: phase >5% slower than baseline
- `[ALERT]` Alert: phase >10% slower (exits with code 1)
- `[OK]` Improvement: phase >5% faster

### `analyze-etl.py`

Analyzes WPR/ETW trace files to produce CPU profiles, context-switch analysis,
and scheduling insights without requiring WPA.

```text
python analyze-etl.py TRACE.etl [--symbols] [--sections SECTIONS] [--process NAMES] [--json PATH] [--folded PATH] [--stacks]
```

| Argument | Required | Description |
| --- | --- | --- |
| `etl` | Yes | Path to .etl trace file |
| `--symbols` | No | Enable symbol resolution (resolves hex to function names) |
| `--sections` | No | Comma-separated analysis sections: cpu, cswitch, sched, timeline (default: all) |
| `--process` | No | Comma-separated process names (default: nanvix-bench.exe,nanvixd.exe) |
| `--json` | No | Write JSON report to this path |
| `--folded` | No | Write folded stacks for flamegraph.pl (implies --symbols) |
| `--stacks` | No | Kernel stack analysis: merge, butterfly, module/function/WHP breakdown (implies --symbols) |
| `--min-hits` | No | Minimum hit count for butterfly stacks (default: 5, only with --stacks) |
| `--xperf` | No | Path to xperf.exe (auto-detected if omitted) |
| `--dump-file` | No | Use pre-existing xperf text dump instead of running xperf |

**Symbol resolution:** The `--symbols` flag resolves kernel addresses to function
names (e.g., `ntoskrnl.exe!MiResolveProtoPteFault` instead of `ntoskrnl.exe!0xfffff807d50b129c`).
Requires environment variables:

```powershell
$env:_NT_SYMBOL_PATH = "srv*C:\Symbols*https://msdl.microsoft.com/download/symbols;<nanvix-repo>\target\release"
$env:_NT_SYMCACHE_PATH = "C:\SymCache"
```

**Analysis sections:**

- **cpu**: CPU sampling profile -- hot modules and functions, kernel/user split
- **cswitch**: Context switch analysis -- switch intervals, preemption sources
- **sched**: Thread scheduling -- ready events count
- **timeline**: Process lifecycle -- creation times and durations

**Requires:** Windows Performance Toolkit (xperf.exe)

### `wpr-profile.wprp`

Lightweight WPR profile designed for Hyper-V/WHP workloads. Captures
essential events with minimal overhead (~1% measured on cold-start benchmark).

**System keywords:**

- CpuConfig, SampledProfile, CSwitch, ReadyThread, ProcessThread
- Loader, HardFaults, DPC, Interrupt

**ETW providers:**

- Hyper-V Hypervisor: VM exits, intercepts, partition ops
- Hyper-V VID: GPA mapping, memory management
- Kernel-Process: process create/exit
- Kernel-Memory: VirtualAlloc/Free for GPA backing
- Kernel-File: file reads during kernel/initrd loading
- Kernel-Interrupt: ISR events during VM exit handling
- Kernel-DPC: deferred procedure calls during exit/re-entry

**Stack walks:** SampledProfile, CSwitch, HardFault, ImageLoad (4 total)

**Usage:**

```powershell
wpr -start scripts\bench\wpr-profile.wprp!NanvixBench -filemode
.\bin\nanvix-bench.exe -benchmark cold-start
wpr -stop trace.etl "Nanvix benchmark"
```

## 6. Understanding the Output

### Phase Breakdown

The report shows per-phase timing at p50/p95/p99 percentiles:

```text
Phase                    p50 (us)   p95 (us)   p99 (us)
------------------------------------------------------
channel_setup                  10         18         21
partition_create             1799       2119       2504
vmem_create                  3175       3608       3759
...
total                       59206      61852      62578
```

### Distribution Analysis (from PERF_TIMINGS)

When built with `PROFILER=yes`, the binary emits per-iteration JSON on stderr.
The analysis tool computes distribution statistics:

```text
Phase                      mean   stddev   CV%      min      max    n
----------------------------------------------------------------------
channel_setup                12        4  33.3%       8       25   50
partition_create           1850      180   9.7%    1650     2500   50
...
```

**CV% (Coefficient of Variation)** indicates measurement stability:

- <5%: Very stable -- reliable measurement
- 5-15%: Normal -- acceptable for benchmarks
- >15%: Noisy -- consider more iterations or system tuning

### Bottleneck Analysis

Identifies top 3 phases consuming the most time:

```text
1. guest_exec: 45,955 us (76.5% of total)
2. exit_handling: 7,307 us (12.2% of total)
3. vmem_create: 3,175 us (5.3% of total)
```

### PERF_TIMINGS Data Format

When built with `PROFILER=yes` (via `.\z.ps1 build --profile --release -- LOG_LEVEL=panic`),
each benchmark iteration emits a JSON line on stderr:

```text
PERF_TIMINGS:{"channel_setup":10,"partition_create":1799,"vmem_create":3175,...,"total":59206}
```

All values are in microseconds. The fields match the phase names in the benchmark
text output. This data enables per-iteration analysis that goes beyond the
aggregated p50/p95/p99 in the text output.

## 7. Guest Flamegraph Profiling

For guest CPU flamegraph profiling — a host-side sampling profiler that captures
guest stack traces from inside the user VM — see the
**[Guest Flamegraph Profiling Guide](profiling-flamegraph.md)**.
