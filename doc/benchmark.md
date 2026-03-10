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

1. `boot-time`: measure the time to start a user VM (excluding `nanvixd`).
1. `cold-start`: measure the latency to start a linuxd and a user VM from scratch and send an HTTP echo to the guest.
1. `cold-start-l2`: same as `cold-start`, but deploy linuxd inside an L2 VM.
1. `cold-start-uvm`: same as `cold-start`, but reuse an existing linuxd instance.
1. `concurrent`: same as `cold-start`, but keep the (one) linuxd and (many) user VM instances alive after each iteration.
1. `concurrent-l2`: same as `concurrent`, but deploy the one linuxd instance inside an L2 VM.
1. `echo-breakdown`: breakdown the contribution of each step in the data-path when sending an HTTP echo (requires re-compilation with `TIMESTAMP_MSG=yes`).
1. `round-trip-latency`: measure the latency as we increase the size of the HTTP echo payload.
1. `warm-start`: measure only the latency to send a fixed-size HTTP echo.
1. `warm-start-vmm`: same as above, but excluding `nanvixd`.

You may see all the optional flags with:

```bash
./bin/nanvix-bench.elf -help
```

Most importantly, if you are pinning cores, make sure to also pass the path to your JSON config file:

```bash
./bin/nanvix-bench.elf -benchmark <benchmark> -hwloc <path_to_file.json>
```

> ℹ️ **Note:** All benchmarks require compiling Nanvix with `RELEASE=yes` and `LOG_LEVEL=panic`.
> ℹ️ **Note:** If you are running the benchmarks with a high number of iterations, consider setting high system limits in the process spawning `nanvix-bench.elf` (i.e. `ulimit -u` and `ulimit -n`).

## Syscall Transport Microbenchmark (`syscall-bench-nostd`)

The `nanvix-bench` utility measures end-to-end application benchmarks. For the VMBus-to-ring work we
also maintain a lower-level guest benchmark in `src/benchmarks/syscall-bench-nostd/` that measures
the syscall transport itself.

### What It Measures

- `fcntl(F_GETFL)` on `STDOUT_FILENO` to force a real linuxd-backed round trip.
- `write()`, `read()`, `pwrite()`, and `pread()` payload sweeps, covering both sequential and
  positioned traffic.
- Legacy microvm transport (`microvm` without `ring-buffer`) versus the ring microvm transport
  (`microvm ring-buffer`).
- Historical fixed-size RTT rows for the original ring path, the CQ-interrupt-suppressed path, and
  the current direct-`linuxd` SQ/CQ path.
- The payload benchmark program now emits `write()`, `read()`, `pwrite()`, and `pread()`
  size sweeps through `65536` bytes; the tables below summarize the latest full rerun.

### What It Does **Not** Measure

- Tier 2 adaptive polling or Tier 3 full polling. Those paths are not implemented yet.
- A single end-to-end zero-copy host syscall path. The fixed-buffer rerun removes the older bulk
  payload bounce on the ring path, but the host still pays for linuxd syscall execution and CQ
  completion handling.
- Calibrated wall-clock nanoseconds. The benchmark converts TSC cycles to nanoseconds assuming an
  approximately 2 GHz guest TSC, so ratios between variants are more trustworthy than the absolute
  nanosecond values.

### Methodology Used for the Results Below

The project-wide recommendation for benchmark builds is still:

```bash
./z build -- all RELEASE=yes LOG_LEVEL=panic
```

The data below was collected with direct Cargo invocations because `./z build` was not usable in the
authoring environment. The commands used for the transport-specific artifacts were:

```bash
# Host-side supervisor.
cargo build -p nanvixd --release

# Legacy uservm.
cargo build -p uservm --release --no-default-features --features microvm

# Ring uservm.
cargo build -p uservm --release --no-default-features --features 'microvm ring-buffer'

# Legacy kernel.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -Zjson-target-spec \
  --manifest-path src/kernel/Cargo.toml \
  --target build/targets/x86_64-kernel.json \
  --release --no-default-features --features microvm

# Ring kernel.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Z build-std=core,alloc,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  -Zjson-target-spec \
  --manifest-path src/kernel/Cargo.toml \
  --target build/targets/x86_64-kernel.json \
  --release --no-default-features --features 'microvm ring-buffer'

# Fixed-size RTT guest benchmark.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec \
  --release \
  --target build/targets/x86_64-user.json \
  -p syscall-bench-nostd \
  --no-default-features \
  --features panic

# Legacy payload-only guest benchmark.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec \
  --release \
  --target build/targets/x86_64-user.json \
  -p syscall-bench-nostd \
  --no-default-features \
  --features 'panic payload-sweep-only'

# Ring payload-only guest benchmark.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec \
  --release \
  --target build/targets/x86_64-user.json \
  -p syscall-bench-nostd \
  --no-default-features \
  --features 'panic payload-sweep-only ring-buffer'
```

Runtime setup and collection procedure:

1. Prepare two runtime trees, one for the legacy transport and one for the ring transport.
2. Install the matching `kernel.elf` and `uservm.elf` into each tree, plus the common
   `nanvixd.elf`, `linuxd.elf`, and `syscall-bench-nostd.elf`.
3. For the payload sweeps, create `syscall-bench-payload.tmp` as a symlink to `/dev/zero` inside
   each `bin/` directory before every run. The benchmark unlinks this path at the end, so it must
   be recreated for the next trial. This makes `write()` / `pwrite()` discard bytes and makes
   `read()` / `pread()` return zero-filled bytes, isolating syscall and transport overhead instead
   of measuring host filesystem work.
4. Run the benchmark from inside each `bin/` directory because `nanvixd` resolves the guest
   program relative to the current working directory:

   ```bash
   cd /tmp/nanvix-bench/<variant>/bin
   ln -sf /dev/zero syscall-bench-payload.tmp
   ./nanvixd.elf \
     -bin-dir /tmp/nanvix-bench/<variant>/bin \
     -log-dir /tmp/nanvix-bench/<variant>/logs-<run> \
     -- syscall-bench-nostd.elf
   ```

5. Parse the `guest_*.log` file and extract:
   - `BENCH ...` lines for the fixed-size RTT benchmark.
   - `SIZEBENCH ...` lines for the payload sweeps.
6. Summarize the payload sweeps by taking the median of the per-trial average nanoseconds.

For tighter fixed-size RTT reruns, do one warm-up run per transport before the measured trials and
pin `nanvixd.elf` (and its children) to a lightly loaded CPU with `taskset -c <cpu> ...`. The
latest direct-linuxd RTT row below uses that warm-up + pinned-core procedure.

The concrete parameters used in the payload sweeps were:

- Payload backend: `/dev/zero` via the `syscall-bench-payload.tmp` symlink.
- Payload sweeps: `write()`, `read()`, `pwrite()`, and `pread()`.
- Payload sizes: `32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768, 65536`
  bytes.
- Warmup iterations per size: `4`.
- Measured iterations per size:
  - `32` iterations for sizes up to and including `4096` bytes.
  - `16` iterations for `8192`, `16384`, `32768`, and `65536` bytes.
- Trials per transport for payload sweeps: `3`.
- Trials per transport for the fixed `fcntl(F_GETFL)` RTT benchmark: `5`.

### Fixed-Size RTT Results (`fcntl(F_GETFL)`)

These runs compare a single linuxd-backed syscall across the main ring-transport milestones so far:
the original ring path, the CQ-interrupt-suppressed hybrid path, and the newer direct-linuxd path.

| Run Set | Legacy Median | Ring Median | Ring / Legacy | Notes |
|---------|---------------|-------------|---------------|-------|
| Before CQ interrupt suppression | `454885 ns` | `621284 ns` | `1.366x` | Ring path injected a guest IRQ for every CQE. |
| After CQ interrupt suppression | `391014 ns` | `424155 ns` | `1.085x` | Current Tier 1 ring path; host only injects when the CQ transitions from empty to non-empty while `CQ_NOTIFY_ME` is armed. |
| Direct linuxd SQ/CQ path | `241275 ns` | `113058 ns` | `0.469x` | Fresh 5-trial interleaved rerun with one warm-up run per transport and a pinned `nanvixd` core. `linuxd` drains SQEs and posts hot-path CQEs directly. |

### Payload Sweep Results (`write()`)

Sequential `write()` now uses the same multi-page fixed-buffer transport as the positioned path:
the guest gathers the user buffer into up to `16` shared fixed buffers, and `linuxd` issues a
single `writev()` per logical transfer. The table below reports 3-trial medians of the per-trial
average latency with `/dev/zero` as the linuxd-side backend.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.747 ms` | `0.391 ms` | `0.523x` |
| 64 | `1.042 ms` | `0.380 ms` | `0.364x` |
| 128 | `0.619 ms` | `0.435 ms` | `0.703x` |
| 256 | `0.620 ms` | `0.461 ms` | `0.745x` |
| 512 | `0.775 ms` | `0.477 ms` | `0.615x` |
| 1024 | `0.534 ms` | `0.393 ms` | `0.735x` |
| 1536 | `0.511 ms` | `0.351 ms` | `0.687x` |
| 2048 | `0.603 ms` | `0.346 ms` | `0.573x` |
| 4096 | `1.533 ms` | `0.394 ms` | `0.257x` |
| 8192 | `1.920 ms` | `0.404 ms` | `0.210x` |
| 16384 | `4.001 ms` | `0.360 ms` | `0.090x` |
| 32768 | `5.519 ms` | `0.374 ms` | `0.068x` |
| 65536 | `12.461 ms` | `0.541 ms` | `0.043x` |

### Payload Sweep Results (`read()`)

Sequential `read()` uses the same multi-page descriptor flow in the opposite direction: `linuxd`
fills the shared fixed buffers with one `readv()`, then the guest scatters those bytes back into
the caller's user pages as the CQEs arrive.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.725 ms` | `0.413 ms` | `0.569x` |
| 64 | `1.027 ms` | `0.362 ms` | `0.352x` |
| 128 | `1.199 ms` | `0.348 ms` | `0.290x` |
| 256 | `0.676 ms` | `0.394 ms` | `0.582x` |
| 512 | `0.776 ms` | `0.350 ms` | `0.450x` |
| 1024 | `0.827 ms` | `0.413 ms` | `0.500x` |
| 1536 | `0.870 ms` | `0.349 ms` | `0.401x` |
| 2048 | `0.706 ms` | `0.431 ms` | `0.611x` |
| 4096 | `1.462 ms` | `0.438 ms` | `0.300x` |
| 8192 | `2.267 ms` | `0.339 ms` | `0.150x` |
| 16384 | `4.510 ms` | `0.945 ms` | `0.209x` |
| 32768 | `8.081 ms` | `2.436 ms` | `0.301x` |
| 65536 | `13.943 ms` | `5.459 ms` | `0.392x` |

### Payload Sweep Results (`pwrite()`)

The positioned write benchmark uses the same shared fixed-buffer scheme, but drives
`linuxd` through `pwritev()` so the transport cost is isolated from file-position updates.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.721 ms` | `0.434 ms` | `0.602x` |
| 64 | `0.697 ms` | `0.366 ms` | `0.525x` |
| 128 | `0.692 ms` | `0.338 ms` | `0.488x` |
| 256 | `0.826 ms` | `0.367 ms` | `0.444x` |
| 512 | `0.924 ms` | `0.405 ms` | `0.439x` |
| 1024 | `0.719 ms` | `0.498 ms` | `0.693x` |
| 1536 | `0.691 ms` | `0.501 ms` | `0.726x` |
| 2048 | `0.754 ms` | `0.388 ms` | `0.515x` |
| 4096 | `1.459 ms` | `0.428 ms` | `0.294x` |
| 8192 | `1.982 ms` | `0.390 ms` | `0.197x` |
| 16384 | `3.239 ms` | `0.426 ms` | `0.131x` |
| 32768 | `4.978 ms` | `0.356 ms` | `0.071x` |
| 65536 | `10.424 ms` | `0.606 ms` | `0.058x` |

### Payload Sweep Results (`pread()`)

`pread()` uses the same fixed-buffer scheme in the opposite direction: `linuxd` copies directly
into the shared ring buffer via `preadv()`, and the guest copies back into the caller's buffer when
the CQEs arrive.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.671 ms` | `0.431 ms` | `0.643x` |
| 64 | `0.642 ms` | `0.461 ms` | `0.718x` |
| 128 | `0.617 ms` | `0.413 ms` | `0.668x` |
| 256 | `0.634 ms` | `0.508 ms` | `0.802x` |
| 512 | `0.647 ms` | `0.472 ms` | `0.729x` |
| 1024 | `0.637 ms` | `0.479 ms` | `0.752x` |
| 1536 | `0.625 ms` | `0.605 ms` | `0.968x` |
| 2048 | `0.725 ms` | `0.425 ms` | `0.586x` |
| 4096 | `1.711 ms` | `0.349 ms` | `0.204x` |
| 8192 | `1.922 ms` | `0.447 ms` | `0.233x` |
| 16384 | `6.165 ms` | `0.895 ms` | `0.145x` |
| 32768 | `5.832 ms` | `2.466 ms` | `0.423x` |
| 65536 | `10.020 ms` | `5.339 ms` | `0.533x` |

### Interpretation

- CQ interrupt suppression substantially improved the original hybrid RTT benchmark: the ring path
  moved from `1.366x` slower than legacy to `1.085x` slower on a fresh 5-trial rerun.
- The direct-linuxd RTT rerun then pushed the fixed-size benchmark past parity: on the latest
  warm-up + pinned-core 5-trial interleaved run, `fcntl(F_GETFL)` improved from `0.241 ms` legacy
  to `0.113 ms` ring (`0.469x` ring / legacy).
- The `/dev/zero` backend removes host ext4/page-cache work from the payload sweep, so these
  numbers are a better measure of syscall + transport overhead than the earlier regular-file runs.
- Ring now beats legacy at every measured size for all four operations (`write()`, `read()`,
  `pwrite()`, and `pread()`) through the new `65536`-byte cap.
- The send-side operations benefit the most from the multi-buffer direct path: at `65536` bytes,
  `write()` drops from `12.461 ms` to `0.541 ms`, and `pwrite()` drops from `10.424 ms` to
  `0.606 ms`.
- The receive-side operations also improve substantially above one page, but the gains are smaller
  because the host-to-guest path still pays for CQ completion handling and guest scatter-back into
  the caller's buffer. At `65536` bytes, `read()` improves from `13.943 ms` to `5.459 ms`, and
  `pread()` improves from `10.020 ms` to `5.339 ms`.
- These payload improvements are consistent with removing the `uservm` SQ-drain/CQ-write hot path
  from the active transport path and amortizing one logical transfer across up to `16` shared fixed
  buffers. Tier 2 adaptive polling, guest-to-host doorbell suppression, and full fallback removal
  are still pending.
- The fixed-size benchmark is still sensitive to host noise, but the warm-up + pinned-core rerun
  brought the absolute RTTs back much closer to the earlier sub-millisecond baseline.
- The smallest payload points can still show shared-environment noise, so the interleaved medians
  and ring/legacy ratios are more reliable than any single raw timing.

The latest rerun and plotting step generated:

- `benchmark-results/write-latency-vs-size-dev-zero.png`
- `benchmark-results/read-latency-vs-size-dev-zero.png`
- `benchmark-results/pwrite-latency-vs-size-dev-zero.png`
- `benchmark-results/pread-latency-vs-size-dev-zero.png`
- `benchmark-results/write-ring-over-legacy-dev-zero.png`
- `benchmark-results/read-ring-over-legacy-dev-zero.png`
- `benchmark-results/pwrite-ring-over-legacy-dev-zero.png`
- `benchmark-results/pread-ring-over-legacy-dev-zero.png`
- `benchmark-results/fcntl-rtt-history.png`
- `benchmark-results/fcntl-rtt-direct-trials.png`

The refreshed raw tables, summaries, and plots from this rerun live under `benchmark-results/`,
including `results-direct-linuxd.tsv`, `results-direct-linuxd-summary.tsv`,
`payload-size-results-dev-zero.tsv`, and `payload-size-summary-dev-zero.tsv`.

When reproducing the benchmark, prefer the median trend and the ratio tables over any single trial:
the experiments ran in a shared development environment, so larger payload points can show visible
run-to-run variance.
