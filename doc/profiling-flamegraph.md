# Guest Flamegraph Profiling (Windows / WHP)

The guest flamegraph profiler is a host-side sampling profiler that captures
guest stack traces while a user VM is running. It periodically cancels the
vCPU (~1 kHz), reads the guest `EIP`/`EBP`/`CR3` registers, and walks the
frame-pointer chain through host-mapped guest memory. After the VM exits, the
collected samples are resolved against ELF symbol tables and written as
**folded stacks** — the standard input format for
[Brendan Gregg's FlameGraph tools](https://github.com/brendangregg/FlameGraph).

> ℹ️ **Note:** The guest profiler currently requires the WHP backend (Windows only).

## Prerequisites

| Requirement | Details |
| --- | --- |
| Windows 11 with WHP enabled | The profiler uses `WHvCancelRunVirtualProcessor` for sampling |
| FlameGraph tools | Clone <https://github.com/brendangregg/FlameGraph> into `tools/FlameGraph` (or anywhere on disk) |
| Perl | Required by `flamegraph.pl`; Git for Windows bundles one at `C:\Program Files\Git\usr\bin\perl.exe` |

## Step 1 — Build with Profiling Enabled

Build with `--profile` **and** override the Cargo `strip` setting so guest
binaries keep their `.symtab` sections (the release profile strips symbols by
default):

```powershell
$env:CARGO_PROFILE_RELEASE_STRIP = "false"
.\z.ps1 build --profile -- all LOG_LEVEL=panic
```

The `--profile` flag implies `RELEASE=yes` and `PROFILER=yes`. Setting
`CARGO_PROFILE_RELEASE_STRIP=false` ensures the kernel and user ELF binaries
retain their symbol tables and frame-pointer chains needed for accurate stack
walking and symbol resolution.

## Step 2 — Set the Output Path and Symbol Files

The profiler is activated by setting the `NANVIX_GUEST_PROFILE_PATH`
environment variable to the desired output file path. Symbol resolution uses
the `.symtab` sections of the kernel and user ELF binaries pointed to by
`NANVIX_KERNEL_SYMBOLS` and `NANVIX_USER_SYMBOLS`:

```powershell
$env:NANVIX_GUEST_PROFILE_PATH = "guest_profile.folded"
$env:NANVIX_KERNEL_SYMBOLS     = "bin\kernel.elf"
$env:NANVIX_USER_SYMBOLS       = "bin\<your-app>.elf"   # e.g., bin\hello-rust-nostd.elf
```

`NANVIX_KERNEL_SYMBOLS` and `NANVIX_USER_SYMBOLS` are optional. If neither is
set the output will contain raw hex addresses (`0xADDRESS`) instead of function
names.

> ⚠️ **Tip:** When profiling a benchmark with multiple iterations, the folded
> stacks file is overwritten on each VM exit. Either set a unique path per
> iteration or profile a single iteration.
>
> **Benchmark call sites:** The `NANVIX_GUEST_PROFILE_PATH` env var works for
> standalone interactive mode (`nanvixd -- <app>`). Benchmarks in
> `nanvix-bench` have their own `guest_profile_path` field in `UserVmArgs`;
> to profile those, change `guest_profile_path: None` to
> `Some("guest_profile.folded".to_string())` at the relevant call site
> (e.g., `src/utils/nanvix-bench/src/benchmarks/vmm/boot_time.rs`).

## Step 3 — Run the Benchmark or Application

```powershell
# Run an application in standalone interactive mode.
.\bin\nanvixd.exe -- .\bin\hello-rust-nostd.elf
```

On exit, the profiler prints a summary to stderr:

```text
GUEST_PROFILE_SYMBOLS: loaded 376 symbols from bin\kernel.elf (1363500 bytes)
GUEST_PROFILE_SYMBOLS: loaded 80 symbols from bin\hello-rust-nostd.elf (137296 bytes)
GUEST_PROFILE: wrote 2 samples to guest_profile.folded
```

## Step 4 — Generate the Flamegraph SVG

Convert the folded stacks into an interactive SVG:

```powershell
# Using Git-bundled Perl:
& "C:\Program Files\Git\usr\bin\perl.exe" tools\FlameGraph\flamegraph.pl `
    --title "Nanvix Guest CPU Profile" `
    --countname samples `
    guest_profile.folded > guest_profile.svg

# Open in a browser (click bars to zoom, Ctrl+F to search).
start guest_profile.svg
```

**Useful `flamegraph.pl` options:**

| Flag | Effect |
| --- | --- |
| `--width 1800` | Wider SVG for dense call stacks |
| `--inverted` | Icicle graph (root at the top) |
| `--minwidth 0.5` | Show narrower frames (default hides very thin ones) |
| `--colors java` | Alternative color palette |

## Step 5 — Analyze the Folded Stacks (Optional)

The `.folded` file is plain text. You can filter it before generating the flamegraph:

```powershell
# Show only kernel functions.
Select-String "kernel_" guest_profile.folded | ForEach-Object { $_.Line } > kernel_only.folded

# Show only a specific user function.
Select-String "my_function" guest_profile.folded | ForEach-Object { $_.Line } > filtered.folded
```

## How It Works

1. **Sampling** — A dedicated thread calls `WHvCancelRunVirtualProcessor` at
   ~1 kHz, causing the vCPU to exit with an `Interrupted` reason.
2. **Register read** — On each profiler-triggered exit the host reads
   `EIP`, `EBP`, and `CR3` from the stopped vCPU.
3. **Stack walk** — The profiler walks the guest's frame-pointer chain
   (`EBP` → saved EBP → return address) through host-mapped guest memory,
   translating user-space virtual addresses via a manual two-level page-table
   walk (kernel addresses are identity-mapped).
4. **Symbol resolution** — After VM exit, each captured address is resolved
   against the ELF `.symtab` sections loaded from `NANVIX_KERNEL_SYMBOLS` and
   `NANVIX_USER_SYMBOLS`.
5. **Output** — Resolved stacks are folded (identical stacks merged with a
   count) and written to the file specified by `NANVIX_GUEST_PROFILE_PATH`.
