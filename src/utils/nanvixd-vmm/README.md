# nanvixd-vmm

`nanvixd-vmm` runs the **Nanvix guest** (standalone deployment mode) on top of
the **OpenVMM** virtualization stack, while **reusing the real Nanvix host-side
daemons** — `hostfsd` (host filesystem) and `networkd` (host networking) —
instead of re-implementing their wire protocols.

It boots the 32-bit Nanvix microkernel + an initrd application on KVM,
implements the MicroVM port-I/O contract, and bridges guest stdin/stdout to the
host. The crate lives in the **Nanvix** workspace so it can depend directly on
the production daemon crates and the shared `sys`/`syscall`/`config`/`hostfs-api`
types.

## Why a dedicated crate?

The host filesystem and networking protocols are non-trivial (the `hostfsd`
request handler alone is ~2k lines, including multi-part request reassembly).
Re-implementing them in the OpenVMM workspace would duplicate that logic and let
it drift. Instead, this crate:

- consumes the OpenVMM virtualization libraries (`virt`, `virt_kvm`,
  `guestmem`, `vm_topology`, `vmcore`, `x86defs`, `hvdef`, `sparse_mmap`,
  `pal_async`) via **cross-workspace path dependencies**, and
- consumes the Nanvix `hostfsd::HostFsHandler` and `networkd::NetworkDaemon`
  (plus the real IKC/syscall message types) as ordinary workspace dependencies.

The IKC bridge (`src/ikc.rs`) mirrors the dispatch in
`nanvix/src/uservm/src/standalone.rs`: outbound guest messages are routed to
host stdio, to `hostfsd` (when the header is a HostFS op), or to `networkd`
(when addressed to `NETWORKD`). When no mount is configured, HostFS requests get
a synthesized error response so the guest's `vfsd` drains its pending operation
instead of blocking.

## Building

The crate's OpenVMM sources are decoupled from any sibling checkout: every
OpenVMM library is a **git dependency pinned to a specific revision** (see
`Cargo.toml`), and the `[patch.crates-io]` entries they need are mirrored in the
Nanvix workspace root. One external build tool is still required: **`protoc`**
(the Protocol Buffers compiler), used by transitive OpenVMM build dependencies
(e.g. `tdisp_proto` via `prost-build`). Install it with
`apt-get install protobuf-compiler`, set `PROTOC`, or rely on OpenVMM's restored
copy if a sibling checkout is present.

The canonical build goes through the Nanvix build system, as an **opt-in**
target (it is intentionally not part of the default `./z build`, since it
fetches the OpenVMM crates from GitHub — network access is required on the first
build):

```bash
./z build -- all-nanvixd-vmm              # build just this crate (debug)
./z build -- WITH_OPENVMM=yes             # include it in the full `all` build
./z build RELEASE=yes -- WITH_OPENVMM=yes # ... release
```

The Makefile target resolves `protoc` automatically (explicit `PROTOC`, then a
system `protoc`, then OpenVMM's restored copy) and sets `MEMORY_SIZE_BYTES`
(read by the Nanvix `config` build script) for you, dropping the resulting
binaries into `bin/`.

For quick iteration you can also build the crate directly with cargo via the
wrapper, which performs the same `protoc`/`MEMORY_SIZE_BYTES` resolution and
forwards extra args:

```bash
./build.sh             # debug
./build.sh --release   # release
```

The crate is **not** part of the workspace `default-members`, so it does not
affect the default `./z build` (the guest build).

## Usage

```text
nanvixd-vmm [-bin-dir DIR] [-console-file PATH] [-ramfs IMG] \
            [-kernel-args ARGS] [-mount DIR] [-allow-host-networking] \
            ( -http-addr HOST:PORT | -- PROGRAM [ARGS...] )
```

`nanvixd-vmm` is a drop-in for the production `nanvixd` standalone binary and
supports the same two mutually exclusive operating modes:

- **terminal** (interactive): a `PROGRAM` is given after `--`; it is booted as
  the initrd, its `ARGS` become the guest command line, the guest's stdin/stdout
  are bridged to the daemon's stdin/stdout, and the process exits with the
  guest's exit code.
- **http**: `-http-addr HOST:PORT` starts a control server exposing the same
  `NEW`/`KILL` JSON API as `nanvixd` (selected by the `X-NVX-Message-Type`
  header) plus a per-VM **gateway Unix socket** carrying the guest's stdio. The
  gateway path is returned in the `NEW` response; the host-side consumer connects
  to it to exchange stdin/stdout.

Common options:

- `-bin-dir DIR` — directory containing `kernel.elf` (default `./bin`).
- `-mount DIR` — serve the guest's HostFS requests via `hostfsd`, rooted at `DIR`.
- `-allow-host-networking` — serve the guest's networking via `networkd`.
- `-console-file PATH` — route the guest kernel console here (default: stderr).
- `-ramfs IMG`, `-kernel-args ARGS` — as in the Nanvix daemon.

For drop-in parity, the flags `-clh-bin-path`, `-hwloc`, `-log-dir`, and
`-netns-pool-size` are accepted and ignored (they are not meaningful for a
single-vCPU standalone OpenVMM guest); `-l2` is rejected.

Examples:

```bash
# Terminal mode: host filesystem mount test.
nanvixd-vmm -bin-dir ./bin -mount ./bin/mount-test-data -- ./bin/mount-test.initrd

# Terminal mode: host networking test.
nanvixd-vmm -bin-dir ./bin -allow-host-networking -- ./bin/network-rust.initrd

# HTTP mode: serve the NEW/KILL control API on 127.0.0.1:9999.
nanvixd-vmm -bin-dir ./bin -http-addr 127.0.0.1:9999
```

Set `NANVIXD_VMM_LOG=debug` for verbose host-side logging (emitted to stderr).

## Tests

Because `nanvixd-vmm` is a drop-in for `nanvixd.elf`, the canonical test path is
the real Nanvix `nanvix-test` framework driven against it:

```bash
./z build -- all                       # guests + nanvix-test + test images
./z build -- all-nanvixd-vmm           # the OpenVMM daemon (-> bin/nanvixd-vmm.elf)
./bin/nanvix-test.elf test/test-standalone-openvmm.toml
```

`test/test-standalone-openvmm.toml` mirrors `test/test-standalone.toml` (same
`empty`, `http`, and `terminal` cases, including the host mount and networking
cases served by the reused `hostfsd`/`networkd` daemons) but points
`nanvixd_binary_path` at `./bin/nanvixd-vmm.elf`. The `snapshot-restore` /
`snapshot-save-exit` cases are omitted because snapshotting is not supported by
this VMM. This is exactly what the `ci-openvmm` CI job runs.

For quick terminal-only iteration without the full harness, `run-standalone-tests.py`
drives the `terminal`-executor cases from `test/test-standalone.toml` directly
against the built binary:

```bash
cargo build -p nanvixd-vmm           # or ./build.sh
python3 run-standalone-tests.py      # add --verbose to print commands
```

## Benchmarks: OpenVMM vs uservm

`bench-compare.py` produces a side-by-side performance comparison of Nanvix
running on **OpenVMM** (`nanvixd-vmm`) versus its **own uservm VMM**
(`bin/nanvixd.elf`). It drives the *native* `nanvix-bench` standalone
benchmarks with the **same** measurement code for both, selecting the VMM via
the `NANVIX_BENCH_NANVIXD` environment variable the harness honors — so the
comparison is apples-to-apples. The standalone-applicable benchmarks are
`cold-start` (process spawn → boot → first stdin/stdout round trip) and
`vfs-bench` (per-VFS-operation latency on a dense ramfs). A third benchmark,
`warm-start`, is implemented directly by `bench-compare.py`: it boots the echo
guest once and times many host→guest→host stdio round trips, measuring the
**communication latency in and out of the VMM** (the IKC path) end-to-end
through the OS pipe. A fourth, `warm-start-vmm`, is the **white-box** form: it
measures the same round trip *in-process* with no OS pipe, giving parity with
the native Nanvix `warm-start-vmm` micro-benchmark. For OpenVMM it is driven by
the `nanvixd-vmm-warmstart` binary, which boots the guest with an in-process
[`ChannelGuestIo`](src/io.rs) endpoint instead of the host terminal; for uservm
it is the native `nanvix-bench -benchmark warm-start-vmm`. The OS-pipe transport
(absent in white-box) is identical for both VMMs, so either form isolates VMM
IKC overhead.

Build everything in release first (for a fair comparison):

```bash
./z build --release LOG_LEVEL=panic                  # guests + bin/nanvixd.elf (uservm)
src/utils/nanvixd-vmm/build.sh --release             # nanvixd-vmm + nanvixd-vmm-warmstart (OpenVMM)
RELEASE=yes LOG_LEVEL=panic MEMORY_SIZE_BYTES=$((128*1048576)) \
    cargo build -p nanvix-bench --release --no-default-features --features standalone,microvm
```

(The release `nanvix-bench` links uservm in release, needed for a fair white-box
`warm-start-vmm`.) Then run the comparison:

```bash
python3 src/utils/nanvixd-vmm/bench-compare.py                          # all four
python3 src/utils/nanvixd-vmm/bench-compare.py --benchmark warm-start-vmm --iterations 1000
python3 src/utils/nanvixd-vmm/bench-compare.py --benchmark vfs-bench --iterations 1000
```

The `uvm/ovm` column is the uservm÷OpenVMM latency ratio (>1 means OpenVMM is
faster). Interactive/streaming benchmarks (`warm-start`, `vfs-bench`) rely on
this crate streaming host stdin to the guest and blocking guest `read(2)` until
data arrives (matching uservm semantics) — see `src/stdin.rs`.
