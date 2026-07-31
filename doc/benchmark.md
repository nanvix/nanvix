# Nanvix Benchmarks

Nanvix ships with `nanvix-bench` for measuring baseline system performance.

For stable results, pin the benchmark client to an isolated CPU set:

```json
{
    "client_core_str": "0-9"
}
```

You will need to save this JSON file.

`nanvix-bench` currently supports the following benchmarks:

- `boot-time`: measure the time to start a user VM (excluding `nanvixd`).
- `cold-start`: spawn `nanvixd` and a user VM, then complete the first echo round trip.
- `cold-start-uvm`: start a user VM and complete its first standalone gateway echo.
- `snapshot-restore`: compare snapshot restore latency with cold boot.
- `vfs-bench`: measure VFS operation latencies (stat, open/close, read, write, readdir, create/unlink, mkdir/rmdir, rename) inside the guest VM using a FAT32 image loaded into guest memory via the RAMFS region.
- `warm-start-gateway`: measure steady-state round-trip latency through the standalone gateway.
- `warm-start-socket`: measure TCP echo latency through guest networking.
- `warm-start-vmm`: measure raw round-trip latency inside the user VM.

you may see all the optional flags with:

```bash
./bin/nanvix-bench.elf -help
```

most importantly, if you are pinning cores, make sure to also pass the path to your JSON config file:

```bash
./bin/nanvix-bench.elf -benchmark <benchmark> -hwloc <path_to_file.json>
```

> ℹ️ **Note:** All benchmarks require compiling Nanvix with `RELEASE=yes` and `LOG_LEVEL=panic`.
> ℹ️ **Note:** If you are running the benchmarks with a high number of iterations, consider setting high system limits in the process spawning `nanvix-bench.elf` (i.e. `ulimit -u` and `ulimit -n`).

## Profiling and Performance Analysis

For detailed profiling infrastructure on Windows -- including system tuning,
WPR/ETW tracing, flamegraph generation, and A/B regression detection -- see the
**[Profiling Guide (Windows)](profiling-windows.md)**.

## Benchmarking on Windows

On Windows, the cold-start benchmark spawns a fresh `nanvixd` process per iteration and measures
the time from process spawn to the first echo response.

### Windows Defender Exclusion

Windows Defender may quarantine unsigned executables in `bin/`. To prevent this,
run the following in an **elevated** (Administrator) PowerShell:

```powershell
Add-MpPreference -ExclusionPath "C:\path\to\nanvix\bin"
```

This exclusion is recursive and covers all files and subdirectories under `bin/`.

### Building for Benchmarks on Windows

```powershell
.\z.ps1 build -- all RELEASE=yes LOG_LEVEL=panic
```

This builds all components including `nanvix-bench.exe` with the WHP backend.

On native Windows ARM64, `z.ps1` automatically builds the AArch64 guest and ARM64 host binaries.
The explicit equivalent is:

```powershell
.\z.ps1 build -- all TARGET=aarch64 RELEASE=yes LOG_LEVEL=panic
```

### Available Benchmarks on Windows

| Benchmark            | Description                                            |
| -------------------- | ------------------------------------------------------ |
| `boot-time`          | Start a user VM (no nanvixd)                           |
| `cold-start`         | Spawn nanvixd + VM + echo round-trip                   |
| `cold-start-uvm`     | Start a user VM + first gateway echo                   |
| `snapshot-restore`   | Compare snapshot restore latency with cold boot         |
| `vfs-bench`          | VFS operation latencies (FAT32 image via RAMFS region) |
| `warm-start-gateway` | Round-trip latency through the standalone gateway      |
| `warm-start-socket`  | TCP echo latency through guest networking              |
| `warm-start-vmm`     | Raw round-trip latency inside the user VM              |

### Running Benchmarks on Windows

```powershell
# Using z.ps1.
.\z.ps1 bench -- -benchmark cold-start -iterations 10

# Or directly.
.\bin\nanvix-bench.exe -benchmark cold-start -iterations 10
.\bin\nanvix-bench.exe -benchmark boot-time -iterations 100
.\bin\nanvix-bench.exe -help
```

Use `-help` to list all benchmark options supported by the current build.

### Architecture-Aware Benchmark Results

The automation script detects the native host architecture and keeps histories separate:

- X64 results use filenames ending in `_X64.csv`.
- ARM64 results use filenames ending in `_ARM64.csv`.

```powershell
python .\scripts\benchmark.py run `
  --benchmark boot-time `
  --machine-type microvm `
  --iterations 100 `
  --output-dir .
```

This prevents ARM64 measurements from being appended to Windows X64 baselines while preserving the
same benchmark names and result schema.
