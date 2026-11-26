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

- `boot-time`: measure a user VM's complete boot, noop execution, and teardown (excluding `nanvixd`).
- `cold-start`: spawn `nanvixd` and a user VM, then complete the first echo round trip.
- `cold-start-uvm`: start a user VM and complete its first standalone gateway echo.
- `snapshot-restore`: compare snapshot restore latency with cold boot.
- `syscall-bench`: measure system call latencies inside the guest VM, both for calls that the kernel serves alone (`getppid`, `clock_gettime`, `sched_yield`) and for calls that system daemons serve (`getpgrp`, `fstat`, `stat`, `read`, `write`, `open`/`close`). Latencies are reported in nanoseconds per call.
- `vfs-bench`: measure the latencies of file system operations (stat, open/close, read, write, readdir, create/unlink, mkdir/rmdir, rename) as a guest process sees them. Every operation goes through the kernel and `vfsd`, which mounts a FAT32 image as the root file system. Results are reported separately for a writable and a read-only mount.
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

## Known KVM Teardown Cost (Linux)

`boot-time` stops its clock after the user VM task completes and its output and control channels
are drained. It therefore includes worker shutdown and destruction of the VMM, not just guest
startup. The inter-iteration cleanup sleep is outside the measurement. The `PerfTimings.total_us`
counter is recorded before the remaining VM handles are dropped, so it is not a substitute for
this end-to-end measurement.

Dropping the last reference to a KVM VM can take tens of milliseconds even without guest RAM.
This is synchronous host-kernel work, not simply Rust heap deallocation: closing the last VM or
vCPU handle can trigger KVM destruction. Nanvix also duplicates the VM descriptor for its IKC
notifier, so the expensive close need not occur when the main `VmFd` field is dropped.
The Linux [KVM destruction path](https://github.com/torvalds/linux/blob/v6.18/virt/kvm/kvm_main.c)
releases interrupt routing, devices, MMU notifiers, and memory slots, and drains SRCU callbacks.
The exact cost depends on the host kernel, scheduling, and resources attached to the VM.

### Investigation Results

A Linux KVM control experiment on WSL2 (`6.18.58.1-microsoft-standard-WSL2`, x86-64) created an
IRQ chip and one vCPU, optionally added a PIT and 256 MiB memory slot, then separately timed
vCPU close, memory unmapping, and VM close. No guest code ran. Twenty iterations per case on a
non-isolated host produced these medians:

| PIT | Guest RAM | vCPU close | Memory unmap | Final VM close |
| --- | --- | --- | --- | --- |
| No | None | 1.6 us | N/A | 31.1 ms |
| Yes | None | 1.8 us | N/A | 35.3 ms |
| Yes | 256 MiB, untouched | 10.2 us | 14.3 us | 36.3 ms |
| Yes | 256 MiB, written throughout | 12.7 us | 5.0 ms | 31.5 ms |

Sampling the reproducer's kernel wait channel also observed `rcu_barrier` and
`__synchronize_srcu.part.0`. These controls reproduce the long close without a guest allocation
or PIT, while showing that freeing populated RAM can add a separate cost. They are diagnostic
observations, not portable performance thresholds or bare-metal benchmark results.

### Reproducing the No-Memory Control

On an x86 Linux host with read/write access to `/dev/kvm`, this Python standard-library example
creates an IRQ chip and vCPU but no PIT or guest memory. It times only the final VM close, after
closing the vCPU. The ioctl constants are from Linux's KVM userspace API.

```bash
python3 - <<'PY'
import fcntl
import os
import time

KVM_CREATE_VM = 0xAE01
KVM_CREATE_VCPU = 0xAE41
KVM_CREATE_IRQCHIP = 0xAE60

samples = []
for _ in range(20):
    with open("/dev/kvm", "rb+", buffering=0) as kvm:
        vm_fd = fcntl.ioctl(kvm, KVM_CREATE_VM, 0)
        with os.fdopen(vm_fd, "rb+", buffering=0) as vm:
            fcntl.ioctl(vm, KVM_CREATE_IRQCHIP, 0)
            os.close(fcntl.ioctl(vm, KVM_CREATE_VCPU, 0))
            start = time.perf_counter_ns()
        samples.append((time.perf_counter_ns() - start) / 1_000_000)

samples.sort()
print(f"VM close: p50={samples[10]:.3f} ms, p95={samples[19]:.3f} ms")
PY
```

### Decision

Keep synchronous, ownership-based teardown and treat this as a known Linux KVM limitation.
Reordering Rust fields or changing the guest allocator cannot remove the delay demonstrated by
the no-memory control. Deferring destruction to a background task would move the cost outside
the benchmark while retaining host resources; it would not eliminate the work. Any future
VM-reuse or asynchronous-reclamation design needs separate lifecycle and resource-bound analysis.
Use `warm-start-vmm` for steady-state execution measurements that intentionally reuse a VM.
Do not apply the KVM diagnosis to Windows: its WHP backend has a different destruction path.

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

### Available Benchmarks on Windows

| Benchmark            | Description                                            |
| -------------------- | ------------------------------------------------------ |
| `boot-time`          | Start a user VM (no nanvixd)                           |
| `cold-start`         | Spawn nanvixd + VM + echo round-trip                   |
| `cold-start-uvm`     | Start a user VM + first gateway echo                   |
| `syscall-bench`      | System call latencies (kernel and system daemons)      |
| `vfs-bench`          | VFS operation latencies through vfsd (FAT32 image)     |
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
