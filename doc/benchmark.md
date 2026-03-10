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
- `pwrite()` payload sweeps, which stress the guest-to-host request path.
- `pread()` payload sweeps, which stress the host-to-guest response path.
- Legacy microvm transport (`microvm` without `ring-buffer`) versus the ring microvm transport
  (`microvm ring-buffer`).
- The fixed-size RTT section below still reflects the Tier 1 ring path with CQ interrupt
  suppression, while the payload sweeps below use the fixed-buffer Phase 5e ring path.
- The payload benchmark program now emits `write()`, `read()`, `pwrite()`, and `pread()`
  size sweeps; the committed tables below summarize the last positioned-I/O rerun.

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
| Direct linuxd SQ/CQ path | `531675 ns` | `246793 ns` | `0.464x` | Fresh 5-trial interleaved rerun with one warm-up run per transport and a pinned `nanvixd` core. `linuxd` drains SQEs and posts hot-path CQEs directly. |

### Payload Sweep Results (`pwrite()`)

The latest rerun uses the direct-linuxd fixed-buffer path for positioned writes. Instead of
forwarding SQEs through the older `uservm` drain/CQ path, the guest now submits a fixed shared-ring
buffer descriptor, `linuxd` drains that SQE directly, and the host reads the payload from the
pre-registered buffer. The table below reports 3-trial medians of the per-trial average latency
with `/dev/zero` as the linuxd-side backend.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.471 ms` | `0.535 ms` | `1.137x` |
| 64 | `0.461 ms` | `0.456 ms` | `0.989x` |
| 128 | `0.522 ms` | `0.298 ms` | `0.570x` |
| 256 | `0.459 ms` | `0.290 ms` | `0.631x` |
| 512 | `0.434 ms` | `0.309 ms` | `0.712x` |
| 1024 | `0.457 ms` | `0.292 ms` | `0.640x` |
| 1536 | `0.503 ms` | `0.307 ms` | `0.610x` |
| 2048 | `0.425 ms` | `0.300 ms` | `0.705x` |
| 4096 | `0.900 ms` | `0.596 ms` | `0.661x` |
| 8192 | `1.382 ms` | `0.917 ms` | `0.663x` |
| 16384 | `2.095 ms` | `1.512 ms` | `0.722x` |
| 32768 | `3.855 ms` | `2.546 ms` | `0.660x` |

### Payload Sweep Results (`pread()`)

`pread()` now uses the same fixed-buffer scheme in the opposite direction: linuxd copies directly
into the shared ring buffer and the guest copies back into the caller's buffer when the CQE
arrives. This table uses the same methodology, but exercises the opposite data direction against
`/dev/zero`.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.429 ms` | `0.340 ms` | `0.793x` |
| 64 | `0.518 ms` | `0.311 ms` | `0.600x` |
| 128 | `0.443 ms` | `0.287 ms` | `0.647x` |
| 256 | `0.452 ms` | `0.319 ms` | `0.704x` |
| 512 | `0.475 ms` | `0.307 ms` | `0.645x` |
| 1024 | `0.475 ms` | `0.287 ms` | `0.605x` |
| 1536 | `0.427 ms` | `0.300 ms` | `0.702x` |
| 2048 | `0.524 ms` | `0.262 ms` | `0.501x` |
| 4096 | `0.921 ms` | `0.595 ms` | `0.647x` |
| 8192 | `1.466 ms` | `0.792 ms` | `0.540x` |
| 16384 | `2.398 ms` | `1.411 ms` | `0.588x` |
| 32768 | `4.124 ms` | `2.472 ms` | `0.600x` |

### Interpretation

- CQ interrupt suppression substantially improved the original hybrid RTT benchmark: the ring path
  moved from `1.366x` slower than legacy to `1.085x` slower on a fresh 5-trial rerun.
- The direct-linuxd RTT rerun then pushed the fixed-size benchmark past parity: on the latest
  warm-up + pinned-core 5-trial interleaved run, `fcntl(F_GETFL)` improved from `0.532 ms` legacy
  to `0.247 ms` ring (`0.464x` ring / legacy).
- The `/dev/zero` backend removes host ext4/page-cache work from the payload sweep, so these
  numbers are a better measure of syscall + transport overhead than the earlier regular-file runs.
- With the direct-linuxd path enabled, a `32768`-byte transfer now favors ring in both directions:
  `3.855 ms` legacy vs `2.546 ms` ring for `pwrite()`, and `4.124 ms` legacy vs `2.472 ms` ring
  for `pread()`.
- `pread()` is now faster than legacy at every measured size. `pwrite()` is faster at every size
  except `32` B and is effectively at parity by `64` B.
- These payload improvements are consistent with removing the `uservm` SQ-drain/CQ-write hot path
  from the active transport path. Tier 2 adaptive polling, guest-to-host doorbell suppression, and
  full fallback removal are still pending.
- The fixed-size benchmark is still sensitive to host noise, but the warm-up + pinned-core rerun
  brought the absolute RTTs back much closer to the earlier sub-millisecond baseline.
- The smallest payload points can still show shared-environment noise, so the interleaved medians
  and ring/legacy ratios are more reliable than any single raw timing.

The latest rerun and plotting step generated:

- `benchmark-results/pwrite-latency-vs-size-dev-zero.png`
- `benchmark-results/pread-latency-vs-size-dev-zero.png`
- `benchmark-results/pwrite-ring-over-legacy-dev-zero.png`
- `benchmark-results/pread-ring-over-legacy-dev-zero.png`
- `benchmark-results/fcntl-rtt-history.png`
- `benchmark-results/fcntl-rtt-direct-trials.png`

Committed copies of the raw tables, summaries, and plots from this rerun live under
`benchmark-results/`, including `results-direct-linuxd.tsv`,
`results-direct-linuxd-summary.tsv`, `payload-size-results-dev-zero.tsv`, and
`payload-size-summary-dev-zero.tsv`.

When reproducing the benchmark, prefer the median trend and the ratio tables over any single trial:
the experiments ran in a shared development environment, so larger payload points can show visible
run-to-run variance.
