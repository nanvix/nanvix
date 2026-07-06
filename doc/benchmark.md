# Nanvix Benchmarks

Nanvix ships with a small benchmarking tool, `nanvix-bench`, that you can use to measure the system's baseline performance.

To get the best performance out of Nanvix, we recommend pinning different components to different sets of cores. In particular, we recommend pinning `linuxd`, the user VM, and the client to different core dies, the latter preferably in a different NUMA domain. For example, in a server with 4 CPU dies, each with 5 cores, the following is a good pinning strategy:

```json
{
    "client_core_str": "0-9",
    "linuxd_core_str": "10-14",
    "nanovm_core_str": "15-19"
}
```

You will need to save this JSON file.

`nanvix-bench` currently supports the following benchmarks:

- `boot-time`: measure the time to start a user VM (excluding `nanvixd`).
- `cold-start`: measure the latency to start a linuxd and a user VM from scratch and send an HTTP echo to the guest.
- `cold-start-uvm`: same as `cold-start`, but reuse an existing linuxd instance.
- `concurrent`: same as `cold-start`, but keep the (one) linuxd and (many) user VM instances alive after each iteration.
- `echo-breakdown`: break down the contribution of each step in the data-path when sending an HTTP echo (requires re-compilation with `TIMESTAMP_MSG=yes`).
- `round-trip-latency`: measure the latency as we increase the size of the HTTP echo payload.
- `vfs-bench`: measure VFS operation latencies (stat, open/close, read, write, readdir, create/unlink, mkdir/rmdir, rename) inside the guest VM using a FAT32 image loaded into guest memory via the RAMFS region.
- `warm-start`: measure only the latency to send a fixed-size HTTP echo.
- `warm-start-vmm`: same as above, but excluding `nanvixd`.

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

On Windows, `nanvix-bench` supports a subset of benchmarks in standalone mode. The standalone
cold-start benchmark spawns a fresh `nanvixd` process per iteration in interactive mode and
measures the time from process spawn to the first echo response.

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

This builds all components including `nanvix-bench.exe` with the standalone and WHP features.

### Available Benchmarks on Windows

| Benchmark        | Description                                            |
| ---------------- | ------------------------------------------------------ |
| `boot-time`      | Start a user VM (no nanvixd)                           |
| `cold-start`     | Spawn nanvixd + VM + echo round-trip (standalone mode) |
| `vfs-bench`      | VFS operation latencies (FAT32 image via RAMFS region) |
| `warm-start-vmm` | Raw round-trip latency inside the user VM              |

### Running Benchmarks on Windows

```powershell
# Using z.ps1.
.\z.ps1 bench -- -benchmark cold-start -iterations 10

# Or directly.
.\bin\nanvix-bench.exe -benchmark cold-start -iterations 10
.\bin\nanvix-bench.exe -benchmark boot-time -iterations 100
.\bin\nanvix-bench.exe -help
```

> ℹ️ **Note:** HTTP-based benchmarks (`warm-start`, `round-trip-latency`, `concurrent`,
> and `echo-breakdown`) are Linux-only.
