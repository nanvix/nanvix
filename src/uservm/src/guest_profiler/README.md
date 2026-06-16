# Nanvix Guest Profiler — End-to-End Guide

This document explains how to use the Nanvix guest profiler to generate
flamegraphs that span from guest user-space code through the guest kernel,
the host VMM, and optionally the host OS kernel.

## Architecture

The profiler captures stack traces at two levels, merged into a single
flamegraph with `[GUEST]` and `[HOST]` root frames:

```
┌─────────────────────────────────────────────────────┐
│  [HOST]: Host VMM + OS Kernel                       │
│  nanvixd, vid.sys/ntoskrnl (Windows), KVM (Linux)   │
│  Captured by: ETW/WPR (Windows) / perf record (Linux)│
├─────────────────────────────────────────────────────┤
│  [GUEST]: Guest kernel + user application           │
│  Nanvix kernel, CPython, libc, syscalls             │
│  Captured by: WHv/KVM register read + frame walk    │
└─────────────────────────────────────────────────────┘
```

## How Sampling Works

### Statistical Validity

The profiler uses **periodic sampling** — the standard technique used by
`perf`, VTune, Instruments, and all production profilers.

1. **Periodic time sampling**: A 1 kHz timer fires every ~1 ms from a
   dedicated thread, independent of guest or host execution state. The
   probability of capturing any code path is proportional to the time
   spent in that path.

2. **Overhead**: Each sample requires a VP cancel (WHP) or signal (KVM),
   register reads, and a frame-pointer walk. At 1 kHz over a 3-second
   run, this adds ~3,000 interrupts — typically small relative to guest
   execution time.

3. **Guest sampling**: When the timer interrupts guest execution, the VP
   returns with an `Interrupted` exit and guest registers (EIP/EBP/CR3)
   reflect the exact interrupted state. This is a true point-in-time
   sample of guest execution.

4. **Host sampling**: Host-side profiling (nanvixd VMM code + OS kernel)
   is handled entirely by ETW (Windows) or `perf record` (Linux). These
   tools run concurrently at the same configurable frequency as the guest
   profiler (controlled by `NANVIX_PROFILER_FREQ_HZ`) and provide full
   symbol resolution via PDB (Windows) or DWARF (Linux), including
   kernel code (vid.sys, ntoskrnl, kvm_intel).

5. **Caveats**:
   - The timer is periodic (not randomized/Poisson), which can alias
     with periodic guest activity. For alias-resistant profiling,
     consider using a prime-number frequency like 997 Hz.
   - `thread::sleep(1ms)` precision depends on OS timer resolution.
   - ETW/perf correlation: our profiler uses QPC (Windows) or
     `CLOCK_MONOTONIC_RAW` (Linux) for timestamps, matching the
     respective platform's profiler clock.

### Sample Count Guidelines

| Duration | Samples at 1 kHz | Statistical quality |
|----------|-----------------|---------------------|
| 100 ms   | ~100            | Rough profile       |
| 1 s      | ~1,000          | Good for hot paths  |
| 5 s      | ~5,000          | Reliable profiling  |
| 30 s     | ~30,000         | Production quality  |

For meaningful flamegraphs, aim for at least 1,000 samples. Run the
workload long enough to accumulate sufficient data.

### Guest vs Host Sampling

**Guest sampling** is done by the built-in profiler timer:

- **Windows (WHP)**: Timer thread calls `WHvCancelRunVirtualProcessor`.
  The run loop returns `Interrupted`, reads EIP/EBP/CR3 via
  `WHvGetVirtualProcessorRegisters`, and walks the frame-pointer chain.

- **Linux (KVM)**: *(Planned — see Linux PR)* Timer thread sends
  `SIGUSR2` to interrupt `KVM_RUN`, then reads registers via
  `KVM_GET_REGS` / `KVM_GET_SREGS` for the frame walk.

The guest profiler only captures stacks when the VP was executing guest
code (Interrupted exit). Host profiling is delegated to ETW/perf which
captures the host stack with full symbol resolution.

**Host sampling** is done by ETW (Windows) or `perf record` (Linux),
which runs concurrently at the same frequency as the guest profiler.
The trace data is post-processed by `analyze-etl.py` (Windows) or
`perf script` (Linux) to extract folded stacks filtered to nanvixd.

## Prerequisites

### Tools

```bash
# Flamegraph generation
cargo install inferno rustfilt

# Windows kernel tracing (optional, requires admin)
# Install Windows Performance Toolkit from Windows SDK

# Linux kernel tracing (optional, requires root for kernel stacks)
# Install perf: sudo apt install linux-tools-$(uname -r)
#   or: sudo tdnf install kernel-tools  (Azure Linux)
```

### Frame Pointers

All binaries in the profiled stack must preserve frame pointers for
the stack walker to produce deep call chains. Without frame pointers,
stack walks terminate after 1-2 frames.

## Building for Profiling

Use `.\z build --profile --release` to build everything in profiling
mode. This single command produces binaries with symbols and frame
pointers across the entire Nanvix stack.

### What `--profile --release` does

The `--profile` flag activates the `release-profiling` Cargo profile
(instead of the default `release` profile). This profile inherits all
release optimizations (opt-level=3, LTO, panic=abort) but adds:

| Setting | `release` | `release-profiling` | Why |
|---------|-----------|---------------------|-----|
| `strip` | `true` | `false` | Preserves .symtab (ELF) and PDB symbols (Windows) |
| `debug` | `false` | `"line-tables-only"` | Emits function names in PDB; sufficient for xperf/WPA |

### How symbols are handled per component

```
Component       Built by            Loaded into VM?   Symbol strategy
──────────────  ──────────────────  ────────────────  ──────────────────────────────────
kernel.elf      z build --profile   Yes               .symtab kept, .debug_* stripped
                                                      (objcopy --strip-debug, automated)
                                                      ~60 KB overhead in guest RAM

nanvixd         z build --profile   No (host)         PDB (Windows) / DWARF (Linux)
                                                      alongside the binary

User app (any)  App's build system  Yes               See "Requirements" below
```

### Building nanvix (kernel + nanvixd)

```powershell
cd D:\src\nanvix
.\z build --profile --release -- LOG_LEVEL=panic
```

This produces:
- `bin/kernel.elf` -- `.symtab` preserved, `.debug_*` stripped automatically.
  Only ~60 KB larger than a fully stripped build. Usable directly as
  the symbol file (no separate `.sym` step).
- `bin/nanvixd.exe` + `bin/nanvixd.pdb` (Windows) -- PDB contains function
  symbols for xperf/WPA resolution.

#### Prerequisites for `--profile` builds

The kernel strip step (`objcopy --strip-debug`) requires one of:

- **Windows**: `cargo install cargo-binutils` and
  `rustup component add llvm-tools` (provides `rust-objcopy`).
- **Linux**: `objcopy` from binutils (typically pre-installed).

If neither is found, the build continues but `kernel.elf` will be
~1.4 MB larger (debug sections remain in guest RAM).

Set the symbol path -- `kernel.elf` is its own symbol file:

```powershell
$env:NANVIX_KERNEL_SYMBOLS = "D:\src\nanvix\bin\kernel.elf"
```

### Building guest user applications for profiling

Guest applications compiled with the Nanvix cross-toolchain must:

1. **Preserve frame pointers**: Compile C/C++ code with
   `-fno-omit-frame-pointer` so the stack walker can follow the EBP chain.

2. **Keep an unstripped symbol file**: The stripped binary goes into the
   VM (minimal size). The unstripped copy stays on the host for offline
   symbol resolution.

```powershell
$env:NANVIX_USER_SYMBOLS = "path\to\your-app.elf.sym"
```

### Requirements for any guest user application

| Requirement | How to achieve | Why |
|------------|----------------|-----|
| Frame pointers | `-fno-omit-frame-pointer` (C/C++) | Stack walker reads EBP chain |
| Symbol file | Keep unstripped binary with `.symtab` | Guest profiler resolves addresses offline |
| Set env var | `NANVIX_USER_SYMBOLS=<path>` | Points profiler to symbol file at runtime |

Without `.symtab`, only `.dynsym` (exported symbols) is available —
static/internal functions won't resolve. With `.symtab`, expect ~97%+
resolution.

## Generating a Flamegraph

### Quick Start (all-in-one scripts)

**Windows** -- run as Administrator for full E2E:

```powershell
.\z.ps1 build --profile --release -- LOG_LEVEL=panic
python scripts\bench\flamegraph.py full --guest-elf bin\python.elf
# Or guest-only (no host stacks):
python scripts\bench\flamegraph.py guest --guest-elf bin\python.elf
```

**Linux** -- *(Linux support is added in a follow-up PR)*

```bash
./z build --profile --release -- LOG_LEVEL=panic
python3 scripts/bench/flamegraph.py full --guest-elf bin/python.elf
```

Without admin/root, guest profiling still works -- only the
host OS kernel stacks require elevation.

### Manual Steps

#### 1. Enable the Profiler

Set environment variables:

```powershell
# Windows
$env:NANVIX_GUEST_PROFILE_PATH = "D:\src\profiling-output\guest.folded"
$env:NANVIX_KERNEL_SYMBOLS = "D:\src\nanvix\bin\kernel.elf"
$env:NANVIX_USER_SYMBOLS = "D:\src\cpython\python.elf.sym"
```

```bash
# Linux
export NANVIX_GUEST_PROFILE_PATH=/path/to/output.folded
export NANVIX_KERNEL_SYMBOLS=/path/to/kernel.elf
export NANVIX_USER_SYMBOLS=/path/to/python.elf.sym
```

When `NANVIX_GUEST_PROFILE_PATH` is not set, the profiler is disabled.

#### 2. Run the Workload

```powershell
.\bin\nanvixd.exe -bin-dir bin -ramfs test.img -- python.elf "-B /script.py"
```

nanvixd automatically:
- Starts the guest profiler timer at the configured frequency
- Captures guest stacks via frame-pointer walk
- Starts ETW for host kernel stacks (if admin, Windows only)
- Stops the host session and saves the trace on exit
- Writes folded stacks

Output on stderr:
```
GUEST_PROFILE_SYMBOLS: loaded 465 symbols from kernel.elf
GUEST_PROFILE_SYMBOLS: loaded 35252 symbols from python.elf.sym
GUEST_PROFILE: wrote 105 samples to output.folded
ETW_SESSION: saved ETL to output.folded.etl
```

#### 3. Generate Flamegraph

For guest-only:

```bash
cat output.folded | rustfilt | inferno-flamegraph > flamegraph.svg
```

For unified guest + host, use `flamegraph.py` which
handles merging, demangling, and `[GUEST]`/`[HOST]` prefix injection
automatically.

#### 4. Additional Analysis

**Windows**: Open the ETL in WPA for detailed kernel stack analysis:
```powershell
wpa.exe output.folded.etl
```

**Linux**: Use `perf report` on the perf data:
```bash
perf report -i output.folded.perf.data
```

## Interpreting the Flamegraph

### Reading the SVG

- **X-axis**: Represents the proportion of samples, not time. Wider bars
  = more samples = more time spent.
- **Y-axis**: Call stack depth. Bottom = root (entry point), top = leaf
  (where CPU was actually executing).
- **Colors**: Random; they carry no meaning.
- **`[GUEST]`**: Root frame for guest stacks (Nanvix kernel + user app).
- **`[HOST]`**: Root frame for host stacks (nanvixd VMM + OS kernel).

### What to Look For

1. **Wide leaf functions**: These are where the CPU spends the most time.
   Optimization targets.

2. **Tall narrow stacks**: Deep call chains that aren't expensive. Usually
   fine.

3. **`[HOST]` frames**: Time spent in the host VMM and OS kernel.
   Wide `vid.sys` (Windows) or `kvm_intel` (Linux) frames indicate
   hypervisor overhead.

### Symbol Resolution

**Important**: Always use **unstripped** ELF files for `NANVIX_KERNEL_SYMBOLS`
and `NANVIX_USER_SYMBOLS`. Stripped binaries contain only `.dynsym` (exported
symbols), which misses internal/static functions and drops resolution from
~99% to ~50%.

For CPython, use `python.elf.sym` (the unstripped build output with full
`.symtab`), **not** `python.elf` (which is stripped for deployment):

```
python.elf      14 MB   .dynsym only   12,245 symbols   ~50% resolution
python.elf.sym  48 MB   .symtab        35,252 symbols   ~99% resolution
```

For the Nanvix kernel, `kernel.elf` retains `.symtab` by default (the build
runs `objcopy --strip-debug` which removes debug sections but keeps the
symbol table).

| Symbol format | Meaning |
|--------------|---------|
| `PyDict_SetItem` | Resolved C function name |
| `0x40362942` | Unresolved address (missing from symbol files) |
| `kernel::mm::virt::vmem::Vmm::try_find_user_frame` | Demangled Rust symbol |

Expected resolution rates:
- **With `.symtab`** (unstripped binaries): ~97%+
- **With `.dynsym` only** (stripped binaries): ~50%

## Symbol File Details

The ELF symbol parser supports two symbol table types:

| Section | Contents | Typical count |
|---------|----------|--------------|
| `.symtab` | All symbols including `static` functions | ~35,000 (CPython) |
| `.dynsym` | Only exported/dynamic symbols | ~12,000 (CPython) |

The parser prefers `.symtab` and falls back to `.dynsym`. Both
`STT_FUNC` (regular functions) and `STT_NOTYPE` (assembly entry points)
are included.

## Platform Differences

| Feature | Windows (WHP) | Linux (KVM) |
|---------|--------------|-------------|
| Guest cancel | `WHvCancelRunVirtualProcessor` | `SIGUSR2` *(Linux PR)* |
| Guest regs | `WHvGetVirtualProcessorRegisters` | `KVM_GET_REGS` *(Linux PR)* |
| Host profiling | ETW / WPR (auto-managed) | `perf record` *(Linux PR)* |
| Timestamps | QPC | `CLOCK_MONOTONIC_RAW` *(Linux PR)* |
| Host symbols | `.etl` → `analyze-etl.py` / WPA | `perf.data` → `perf script` *(Linux PR)* |
| E2E script | `flamegraph.py full` | `flamegraph.py full` |

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `NANVIX_GUEST_PROFILE_PATH` | Output path for folded stacks; enables profiler when set | (disabled) |
| `NANVIX_KERNEL_SYMBOLS` | Path to kernel ELF with `.symtab` | (none) |
| `NANVIX_USER_SYMBOLS` | Path to user app ELF with `.symtab` | (none) |
| `NANVIX_PROFILER_FREQ_HZ` | Sampling frequency for both guest and host | `1000` |

## Troubleshooting

### No samples collected
- Ensure `NANVIX_GUEST_PROFILE_PATH` env var is set to an output path
- Ensure the workload runs long enough (>100 ms for meaningful data)

### Shallow stacks (1-2 frames)
- Rebuild with `-fno-omit-frame-pointer` (CPython: `Makefile.nanvix`)
- Ensure `force-frame-pointers = true` in `Cargo.toml` release profile

### Many unresolved addresses
- Use unstripped symbol files (`.symtab`, not just `.dynsym`)
- Set `NANVIX_KERNEL_SYMBOLS` and `NANVIX_USER_SYMBOLS` env vars
- Rebuild symbol files after code changes

### ETW/perf fails to start
- Windows: WPR requires administrator privileges. nanvixd logs
  `"Host kernel tracing failed to start"` and continues without kernel
  stacks. Run as admin for full E2E.
- Linux: *(System-wide `perf record` support is added in the Linux PR.)*
- Both: guest stacks are always captured regardless of host tracing.

### No host stacks in flamegraph
- Windows: Ensure nanvixd ran as admin (check stderr for `ETW_SESSION`)
- Set `_NT_SYMBOL_PATH` (Windows) for OS kernel symbol resolution
- The `flamegraph.py` script handles symbol paths and merging
  automatically

