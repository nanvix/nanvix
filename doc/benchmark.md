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
- The current ring receive path includes the batched fixed-buffer completion optimization for
  `read()` / `pread()`: one host `readv()` / `preadv()` still fills the shared fixed buffers, but
  completion traffic is collapsed to one logical CQ event per syscall.
- The payload benchmark program now emits sequential `write()` / `read()` size sweeps through
  `65536` bytes and positioned `pwrite()` / `pread()` size sweeps through `131072` bytes. The
  tables below still summarize the latest full rerun, which stopped at `65536` bytes for all four
  operations.

### What It Does **Not** Measure

- The bounded guest-to-host SQ polling window that landed after the latest rerun,
  or any future Tier 3 full polling. The published numbers below predate that
  submission-side optimization.
- A single end-to-end zero-copy host-to-guest receive path. The fixed-buffer rerun removes the
  older bulk payload bounce on the ring path, and the new batched receive completion removes the
  old per-segment CQ/control overhead, but the guest still must copy bytes from the shared fixed
  buffers back into the caller's buffer to preserve ownership semantics.
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

### Batched Read Completion (`read()` / `pread()`)

The multi-page fixed-buffer path already let `linuxd` issue one host `readv()` / `preadv()` per
logical syscall, but the original receive path still posted one fixed-buffer completion per 4 KiB
segment. That meant extra CQ writes, guest CQ polls, state lookups, and wakeups even though the
host syscall itself was already batched.

The current protocol keeps the final shared-buffer-to-user-buffer copy, because that copy is still
required to preserve ownership of the caller's bytes. What changed is the completion traffic:
`linuxd` now reports one logical completion with the total transferred length, and the guest kernel
walks the stored segment list locally to copy the bytes back in order and wake the blocked caller
once. On the direct ring path this is encoded as `CqeFlags::BUFFER | CqeFlags::BATCH`; the framed
fallback uses `FixedBufferFlags::COMPLETION_BATCH`.

The trade-off is slightly more protocol/state complexity and a larger framed fixed-buffer
descriptor, but the receive side no longer pays per-segment completion overhead on the hot path.

The concrete parameters used in the payload sweeps were:

- Payload backend: `/dev/zero` via the `syscall-bench-payload.tmp` symlink.
- Payload sweeps: `write()`, `read()`, `pwrite()`, and `pread()`.
- Sequential `write()` / `read()` sizes:
  `32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768, 65536` bytes.
- Positioned `pwrite()` / `pread()` sizes:
  `32, 64, 128, 256, 512, 1024, 1536, 2048, 4096, 8192, 16384, 32768, 65536, 131072` bytes.
- Warmup iterations per size: `4`.
- Measured iterations per size:
  - `32` iterations for sizes up to and including `4096` bytes.
  - `16` iterations for `8192`, `16384`, `32768`, `65536`, and `131072` bytes.
- Trials per transport for payload sweeps: `3`.
- Trials per transport for the fixed `fcntl(F_GETFL)` RTT benchmark: `5`.

### Fixed-Size RTT Results (`fcntl(F_GETFL)`)

These runs compare a single linuxd-backed syscall across the main ring-transport milestones so far:
the original ring path, the CQ-interrupt-suppressed hybrid path, and the newer direct-linuxd path.

| Run Set | Legacy Median | Ring Median | Ring / Legacy | Notes |
|---------|---------------|-------------|---------------|-------|
| Before CQ interrupt suppression | `454885 ns` | `621284 ns` | `1.366x` | Ring path injected a guest IRQ for every CQE. |
| After CQ interrupt suppression | `391014 ns` | `424155 ns` | `1.085x` | Current Tier 1 ring path; host only injects when the CQ transitions from empty to non-empty while `CQ_NOTIFY_ME` is armed. |
| Direct linuxd SQ/CQ path | `241275 ns` | `118293 ns` | `0.490x` | Fresh 5-trial interleaved rerun with one warm-up run per transport and a pinned `nanvixd` core. `linuxd` drains SQEs and posts hot-path CQEs directly. |

### Payload Sweep Results (`write()`)

Sequential `write()` now uses the same multi-page fixed-buffer transport as the positioned path:
the guest gathers the user buffer into up to `16` shared fixed buffers, and `linuxd` issues a
single `writev()` per logical transfer. The table below reports 3-trial medians of the per-trial
average latency with `/dev/zero` as the linuxd-side backend.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.622 ms` | `0.391 ms` | `0.628x` |
| 64 | `0.573 ms` | `0.347 ms` | `0.607x` |
| 128 | `0.567 ms` | `0.459 ms` | `0.809x` |
| 256 | `0.611 ms` | `0.441 ms` | `0.722x` |
| 512 | `0.530 ms` | `0.477 ms` | `0.900x` |
| 1024 | `0.500 ms` | `0.402 ms` | `0.803x` |
| 1536 | `0.485 ms` | `0.384 ms` | `0.791x` |
| 2048 | `0.539 ms` | `0.346 ms` | `0.641x` |
| 4096 | `1.206 ms` | `0.350 ms` | `0.290x` |
| 8192 | `1.684 ms` | `0.330 ms` | `0.196x` |
| 16384 | `2.759 ms` | `0.358 ms` | `0.130x` |
| 32768 | `4.830 ms` | `0.383 ms` | `0.079x` |
| 65536 | `10.157 ms` | `0.613 ms` | `0.060x` |

### Payload Sweep Results (`read()`)

Sequential `read()` uses the same multi-page descriptor flow in the opposite direction: `linuxd`
fills the shared fixed buffers with one `readv()`, then the guest scatters those bytes back into
the caller's user pages after one batched completion arrives for the whole logical transfer.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.577 ms` | `0.336 ms` | `0.583x` |
| 64 | `0.544 ms` | `0.386 ms` | `0.709x` |
| 128 | `0.635 ms` | `0.352 ms` | `0.555x` |
| 256 | `0.542 ms` | `0.359 ms` | `0.662x` |
| 512 | `0.559 ms` | `0.352 ms` | `0.630x` |
| 1024 | `0.593 ms` | `0.341 ms` | `0.575x` |
| 1536 | `0.545 ms` | `0.389 ms` | `0.713x` |
| 2048 | `0.648 ms` | `0.352 ms` | `0.543x` |
| 4096 | `1.359 ms` | `0.331 ms` | `0.244x` |
| 8192 | `1.709 ms` | `0.339 ms` | `0.199x` |
| 16384 | `2.896 ms` | `0.395 ms` | `0.136x` |
| 32768 | `5.527 ms` | `0.378 ms` | `0.068x` |
| 65536 | `11.944 ms` | `0.551 ms` | `0.046x` |

### Payload Sweep Results (`pwrite()`)

The positioned write benchmark uses the same shared fixed-buffer scheme, but drives
`linuxd` through `pwritev()` so the transport cost is isolated from file-position updates.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.632 ms` | `0.331 ms` | `0.524x` |
| 64 | `0.616 ms` | `0.366 ms` | `0.594x` |
| 128 | `0.603 ms` | `0.338 ms` | `0.560x` |
| 256 | `0.741 ms` | `0.369 ms` | `0.497x` |
| 512 | `0.666 ms` | `0.375 ms` | `0.563x` |
| 1024 | `0.719 ms` | `0.269 ms` | `0.374x` |
| 1536 | `0.691 ms` | `0.404 ms` | `0.585x` |
| 2048 | `0.754 ms` | `0.401 ms` | `0.532x` |
| 4096 | `1.459 ms` | `0.358 ms` | `0.245x` |
| 8192 | `1.982 ms` | `0.463 ms` | `0.234x` |
| 16384 | `3.231 ms` | `0.426 ms` | `0.132x` |
| 32768 | `4.978 ms` | `0.388 ms` | `0.078x` |
| 65536 | `10.424 ms` | `0.592 ms` | `0.057x` |

### Payload Sweep Results (`pread()`)

`pread()` uses the same fixed-buffer scheme in the opposite direction: `linuxd` copies directly
into the shared ring buffer via `preadv()`, and the guest copies back into the caller's buffer when
the batched completion arrives.

| Size (bytes) | Legacy Median | Ring Median | Ring / Legacy |
|--------------|---------------|-------------|---------------|
| 32 | `0.558 ms` | `0.365 ms` | `0.655x` |
| 64 | `0.607 ms` | `0.461 ms` | `0.759x` |
| 128 | `0.561 ms` | `0.366 ms` | `0.653x` |
| 256 | `0.564 ms` | `0.463 ms` | `0.821x` |
| 512 | `0.556 ms` | `0.428 ms` | `0.770x` |
| 1024 | `0.576 ms` | `0.395 ms` | `0.686x` |
| 1536 | `0.607 ms` | `0.377 ms` | `0.622x` |
| 2048 | `0.567 ms` | `0.425 ms` | `0.748x` |
| 4096 | `1.155 ms` | `0.349 ms` | `0.302x` |
| 8192 | `1.814 ms` | `0.391 ms` | `0.216x` |
| 16384 | `2.789 ms` | `0.519 ms` | `0.186x` |
| 32768 | `5.404 ms` | `1.025 ms` | `0.190x` |
| 65536 | `9.983 ms` | `1.346 ms` | `0.135x` |

### Interpretation

- CQ interrupt suppression substantially improved the original hybrid RTT benchmark: the ring path
  moved from `1.366x` slower than legacy to `1.085x` slower on a fresh 5-trial rerun.
- The direct-linuxd RTT rerun then pushed the fixed-size benchmark past parity: on the latest
  warm-up + pinned-core 5-trial interleaved run, `fcntl(F_GETFL)` improved from `0.241 ms` legacy
  to `0.118 ms` ring (`0.490x` ring / legacy).
- The `/dev/zero` backend removes host ext4/page-cache work from the payload sweep, so these
  numbers are a better measure of syscall + transport overhead than the earlier regular-file runs.
- Ring now beats legacy at every measured size for all four operations (`write()`, `read()`,
  `pwrite()`, and `pread()`) through the published `65536`-byte cap of the current rerun.
- The send-side operations benefit the most from the multi-buffer direct path: at `65536` bytes,
  `write()` drops from `10.157 ms` to `0.613 ms`, and `pwrite()` drops from `10.424 ms` to
  `0.592 ms`.
- The receive-side operations now improve much more strongly because the host-to-guest path no
  longer pays per-segment CQ completion overhead. The final guest copy back into the caller's
  buffer remains, but at `65536` bytes `read()` still improves from `11.944 ms` to `0.551 ms`, and
  `pread()` improves from `9.983 ms` to `1.346 ms`.
- These payload improvements are consistent with removing the `uservm` SQ-drain/CQ-write hot path
  from the active transport path, amortizing one logical transfer across up to `16` shared fixed
  buffers, and collapsing receive-side completion traffic to one logical CQ event per
  `readv()` / `preadv()` result. The later bounded guest-to-host SQ polling window is not
  reflected in these published numbers; full fallback removal is still pending.
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
