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

# Payload-only guest benchmark.
RUSTFLAGS='-C relocation-model=static -C prefer-dynamic=no' \
cargo +nanvix-x86_64 build \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec \
  --release \
  --target build/targets/x86_64-user.json \
  -p syscall-bench-nostd \
  --no-default-features \
  --features 'panic payload-sweep-only'
```

Runtime setup and collection procedure:

1. Prepare two runtime trees, one for the legacy transport and one for the ring transport.
2. Install the matching `kernel.elf` and `uservm.elf` into each tree, plus the common
   `nanvixd.elf`, `linuxd.elf`, and `syscall-bench-nostd.elf`.
3. For the payload sweeps, create `syscall-bench-payload.tmp` as a symlink to `/dev/zero` inside
   each `bin/` directory before every run. The benchmark unlinks this path at the end, so it must
   be recreated for the next trial. This makes `pwrite()` discard bytes and makes `pread()` return
   zero-filled bytes, isolating syscall and transport overhead instead of measuring host filesystem
   work.
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

The concrete parameters used in the payload sweeps were:

- Payload backend: `/dev/zero` via the `syscall-bench-payload.tmp` symlink.
- Payload sizes: `32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768` bytes.
- Warmup iterations per size: `4`.
- Measured iterations per size:
  - `32` iterations for sizes up to and including `4096` bytes.
  - `16` iterations for `8192`, `16384`, and `32768` bytes.
- Trials per transport for payload sweeps: `3`.
- Trials per transport for the fixed `fcntl(F_GETFL)` RTT benchmark: `5`.

### Fixed-Size RTT Results (`fcntl(F_GETFL)`)

These runs compare a single linuxd-backed syscall before and after CQ interrupt suppression was
implemented in the ring path.

| Run Set | Legacy Median | Ring Median | Ring / Legacy | Notes |
|---------|---------------|-------------|---------------|-------|
| Before CQ interrupt suppression | `454885 ns` | `621284 ns` | `1.366x` | Ring path injected a guest IRQ for every CQE. |
| After CQ interrupt suppression | `391014 ns` | `424155 ns` | `1.085x` | Current Tier 1 ring path; host only injects when the CQ transitions from empty to non-empty while `CQ_NOTIFY_ME` is armed. |

### Payload Sweep Results (`pwrite()`)

The latest rerun uses the fixed-buffer Phase 5e ring path for positioned writes. Instead of
bouncing payload bytes through the older bulk push path, the guest now submits a fixed shared-ring
buffer descriptor and linuxd reads directly from that pre-registered buffer. The table below
reports 3-trial medians of the per-trial average latency with `/dev/zero` as the linuxd-side
backend.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.460 ms` | `0.555 ms` | `1.207x` |
| 64 | `0.496 ms` | `0.535 ms` | `1.078x` |
| 128 | `0.484 ms` | `0.541 ms` | `1.119x` |
| 256 | `0.488 ms` | `0.558 ms` | `1.143x` |
| 512 | `0.495 ms` | `0.561 ms` | `1.135x` |
| 1024 | `0.445 ms` | `0.540 ms` | `1.213x` |
| 1536 | `0.452 ms` | `0.481 ms` | `1.064x` |
| 2048 | `0.496 ms` | `0.494 ms` | `0.994x` |
| 4096 | `0.874 ms` | `0.965 ms` | `1.104x` |
| 8192 | `1.301 ms` | `1.376 ms` | `1.057x` |
| 16384 | `2.272 ms` | `2.584 ms` | `1.137x` |
| 32768 | `4.320 ms` | `4.363 ms` | `1.010x` |

### Payload Sweep Results (`pread()`)

`pread()` now uses the same fixed-buffer scheme in the opposite direction: linuxd copies directly
into the shared ring buffer and the guest copies back into the caller's buffer when the CQE
arrives. This table uses the same methodology, but exercises the opposite data direction against
`/dev/zero`.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.505 ms` | `0.497 ms` | `0.985x` |
| 64 | `0.498 ms` | `0.526 ms` | `1.057x` |
| 128 | `0.470 ms` | `0.498 ms` | `1.060x` |
| 256 | `0.514 ms` | `0.479 ms` | `0.932x` |
| 512 | `0.485 ms` | `0.473 ms` | `0.976x` |
| 1024 | `0.480 ms` | `0.525 ms` | `1.094x` |
| 1536 | `0.476 ms` | `0.503 ms` | `1.057x` |
| 2048 | `0.485 ms` | `0.463 ms` | `0.956x` |
| 4096 | `0.992 ms` | `0.951 ms` | `0.959x` |
| 8192 | `1.606 ms` | `1.479 ms` | `0.921x` |
| 16384 | `2.604 ms` | `2.464 ms` | `0.946x` |
| 32768 | `4.540 ms` | `4.550 ms` | `1.002x` |

### Interpretation

- CQ interrupt suppression substantially improved the fixed-size RTT benchmark: the ring path moved
  from `1.366x` slower than legacy to `1.085x` slower on a fresh 5-trial rerun.
- The `/dev/zero` backend removes host ext4/page-cache work from the payload sweep, so these
  numbers are a better measure of syscall + transport overhead than the earlier regular-file runs.
- With storage effects removed, a `32768`-byte transfer is now effectively at parity in both
  directions: `4.320 ms` legacy vs `4.363 ms` ring for `pwrite()`, and `4.540 ms` legacy vs
  `4.550 ms` ring for `pread()`.
- The fixed-buffer rerun materially narrowed the earlier large-payload gap. `pread()` is now at or
  below legacy through much of the mid/large range (`4096`-`16384` B), while `pwrite()` remains
  modestly slower on most points but stays close to parity from `2048` B upward.
- The smallest points still show visible noise in the shared development environment, so the
  `4096`-byte-and-up trend is the more reliable indicator.
- The remaining write-side gap is consistent with the current architecture: the fixed buffer
  removed the older bulk bounce, but the system still pays for the host drain-thread handoff,
  linuxd processing, and guest CQ completion path. Tier 2 adaptive polling is still pending.

During the local rerun that produced the `/dev/zero` numbers above, the plotting step generated:

- `/tmp/nanvix-bench/pwrite-latency-vs-size-dev-zero.png`
- `/tmp/nanvix-bench/pread-latency-vs-size-dev-zero.png`
- `/tmp/nanvix-bench/pwrite-ring-over-legacy-dev-zero.png`
- `/tmp/nanvix-bench/pread-ring-over-legacy-dev-zero.png`

Committed copies of the raw tables, summaries, and plots from this rerun live under
`benchmark-results/`.

When reproducing the benchmark, prefer the median trend and the ratio tables over any single trial:
the experiments ran in a shared development environment, so larger payload points can show visible
run-to-run variance.
