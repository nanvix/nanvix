# Shared-Memory Ring Buffer VMBus — Design Sketch and Current Status

## Problem

Every guest syscall today costs 3+ KVM exits, 3–4 data copies, and a Unix
socket round-trip. Busy-polling (vhost-user/DPDK style) fixes latency but
wastes CPU cores.

## Goal

Near-native syscall throughput **without** dedicating host cores to polling.

---

## Current Implemented Architecture

The transport that exists in-tree today is no longer fully hybrid, but it is
also not yet the full original end-state. The hot path for `IkcMessage` requests
and `FIXED_BUF` payload transfers is now direct through the shared ring:
`linuxd` drains SQEs from the shared mapping itself and writes the corresponding
CQEs back there. `uservm` still owns the KVM-facing edges (ioeventfd doorbell
receipt and guest IRQ injection), and the older framed socket path remains as a
compatibility fallback for responses that still need `IkcFrame::Bulk`.

- `uservm` creates a temp-file-backed `MAP_SHARED` ring backing file and maps it
  into the guest GPA reserved for the ring.
- The guest kernel writes SQEs into that ring and rings an ioeventfd-backed
  doorbell.
- `uservm` translates the doorbell eventfd wakeup into a shared-memory futex
  signal for `linuxd` instead of draining the SQ itself.
- `linuxd` maps the same backing file, drains `SqeOpcode::IkcMessage` and
  `SqeOpcode::BulkData` + `FIXED_BUF` requests directly from the shared ring,
  and continues to execute syscalls on the existing worker-thread path.
- For hot-path completions, `linuxd` writes CQEs directly into the shared ring:
  regular `Message` responses go into CQ data slots, and fixed-buffer read
  completions use `CqeFlags::BUFFER`; batched receive completions additionally
  set `CqeFlags::BATCH`.
- `uservm` watches a second shared notification word and injects an interrupt
  only when `linuxd` asks for it after posting a CQE while the guest has armed
  `CQ_NOTIFY_ME`.
- The older `uservm`/socket response path remains in place as the compatibility
  path for transfers that still rely on owned `IkcFrame::Bulk` payloads.

### Implemented Paths for Comparison

- **Legacy vmbus baseline**: PMIO envelopes travel through the existing
  `uservm`/`linuxd` channel, and payload bytes move through `push()` / `pull()`
  with `DataChunk` buffers.
- **Previous ring payload design**: the ring carries `SqeOpcode::BulkData`,
  `uservm` drains it and forwards `IkcFrame::Bulk`, but payload bytes still
  bounce through the older bulk path.
- **Current fixed-buffer ring path**: the ring carries `SqeOpcode::BulkData`
  with `FIXED_BUF`, `linuxd` drains it directly from the shared mapping, and
  the payload bytes stay in the ring-backed fixed-buffer region.

---

## Original Direct-Linuxd Architecture (Design Target, for Comparison)

The diagram below is the earlier end-state sketch in which `linuxd` would own
all hot-path draining/completion work directly. The current implementation above
has now reached that shape for `IkcMessage` requests and fixed-buffer payload
transfers, but it still keeps `uservm` responsible for KVM-facing signaling and
still falls back to the older socket/framed path for compatibility cases that
use owned `IkcFrame::Bulk` responses.

```
 ┌──────────────────── Guest (KVM) ────────────────────┐
 │                                                      │
 │  User Process                                        │
 │    write(fd, buf, 128)                               │
 │         │                                            │
 │         ▼                                            │
 │  Kernel: encode SQE in ring ──────┐                  │
 │          maybe ring doorbell      │                  │
 │          HLT / poll CQ           │                  │
 │                                   ▼                  │
 │            ┌─────────────────────────────┐           │
 │            │     Shared Memory Region    │           │
 │            │  (pinned guest phys pages)  │           │
 │            │                             │           │
 │            │  ┌───────────────────────┐  │           │
 │            │  │  Submission Queue     │  │           │
 │            │  │  (guest writes,      │  │           │
 │            │  │   linuxd reads)      │  │           │
 │            │  └───────────────────────┘  │           │
 │            │  ┌───────────────────────┐  │           │
 │            │  │  Completion Queue     │  │           │
 │            │  │  (linuxd writes,     │  │           │
 │            │  │   guest reads)       │  │           │
 │            │  └───────────────────────┘  │           │
 │            │  ┌───────────────────────┐  │           │
 │            │  │  Data Buffers        │  │           │
 │            │  │  (pre-registered,    │  │           │
 │            │  │   zero-copy region)  │  │           │
 │            │  └───────────────────────┘  │           │
 │            └──────────────┬──────────────┘           │
 └───────────────────────────┼──────────────────────────┘
                             │
           same physical pages, two mappings
                             │
 ┌───────────────────────────┼──────────────────────────┐
 │  Host: linuxd             ▼                          │
 │                                                      │
 │  mmap(guest_mem_fd, shared_region_offset)            │
 │  → direct pointer to SQ / CQ / Data Buffers         │
 │                                                      │
 │  ┌─────────────────────────────────────────────┐     │
 │  │  Event Loop (epoll)                         │     │
 │  │                                             │     │
 │  │  wait on:                                   │     │
 │  │    • ioeventfd  (guest rang doorbell)       │     │
 │  │    • timerfd    (adaptive poll window)      │     │
 │  │    • signalfd   (shutdown)                  │     │
 │  │                                             │     │
 │  │  on wake:                                   │     │
 │  │    drain all SQEs from ring                 │     │
 │  │    execute host syscalls                    │     │
 │  │    post CQEs to completion ring             │     │
 │  │    inject guest interrupt (if needed)       │     │
 │  └─────────────────────────────────────────────┘     │
 └──────────────────────────────────────────────────────┘
```

---

## Core Idea: Three Notification Tiers

The three tiers below describe the original notification strategy target. The
current implementation uses the Tier 1 eventfd wakeup path, CQ interrupt
suppression, and a bounded Tier 2-style SQ polling window on the direct-linuxd
path. It does not yet implement the Tier 3 dedicated poll thread or a true
zero-notification steady state.

### Tier 1 — Eventfd Doorbell (default, zero idle CPU)

```
Guest kernel                         linuxd
    │                                   │
    ├─ write SQE to ring               │ (sleeping on epoll)
    ├─ out32(DOORBELL_PORT, 1)         │
    │      │                            │
    │      └─ KVM ioeventfd ──────────► epoll wakes
    │                                   ├─ drain SQ
    │                                   ├─ execute syscalls
    │                                   ├─ post CQEs
    │                                   └─ KVM interrupt inject ──► guest reads CQ
    │
    └─ guest resumes
```

**Status today**: implemented as an ioeventfd-backed wakeup into `uservm`, which
then wakes a `linuxd` SQ worker through a shared-memory futex word. `linuxd`
drains the SQ directly once woken and may keep a short bounded SQ poll window
open before parking again.
**Cost today**: one initial doorbell/eventfd wakeup to enter a burst, suppressed
follow-on guest doorbells while the SQ poll window stays open, plus up to one
guest interrupt injection per batch when the guest has armed `CQ_NOTIFY_ME`.
**CPU when idle**: zero — both sides sleep on epoll/HLT.

KVM ioeventfd is still the crucial primitive here: the guest writes to a PIO
port and KVM signals an eventfd **in-kernel**. In the current implementation,
that eventfd wakes `uservm`, which then wakes `linuxd` through the shared SQ
notification word.

### Tier 2 — Adaptive Polling (implemented as a bounded SQ poll window)

After linuxd drains a batch, it enters a short polling window:

```
linuxd after processing a batch:
    for _ in 0..POLL_SPINS {        // e.g., 1000 iterations (~2-5 μs)
        if sq.tail != sq.head {
            process_batch();
            reset spin counter;
        }
        spin_loop_hint();
    }
    // no new work → fall back to epoll (Tier 1)
```

**Status today**: implemented on the direct-linuxd SQ worker. After draining a
batch, `linuxd` clears `SQ_NEED_WAKEUP`, spins for up to
`RING_POLL_SPIN_ITERS`, and resets the spin budget whenever more SQEs arrive.
When the window expires it re-arms `SQ_NEED_WAKEUP`, re-checks the SQ once to
avoid a lost wakeup race, and only then parks again.
**Current cost**: no additional guest doorbells while the polling window is
open, but idle CPU remains bounded by the short spin window.

### Tier 3 — Dedicated Poll Thread (design target, not yet implemented)

For benchmarks or throughput-critical deployments, pin a host core to
poll the SQ continuously. Same as vhost-user. Enabled by flag, never default.

**Status today**: not implemented.

**Planned automatic tier selection:**

```
             Tier 1 (eventfd)
                  │
    SQ traffic > threshold for N μs
                  │
                  ▼
             Tier 2 (adaptive poll)
                  │
    SQ idle for > POLL_TIMEOUT μs
                  │
                  ▼
             Tier 1 (eventfd)
```

Current behavior uses Tier 1 wakeups plus this bounded Tier 2 poll window on
the direct-linuxd path.

---

## Ring Buffer Layout

### Shared Memory Region (e.g., 2 MiB at a fixed GPA)

```
Offset   Size     Contents
──────   ─────    ────────────────────────────
0x0000   64 B     Control block (head/tail/flags for SQ and CQ)
0x0040   --       (padding to cache line)
0x0080   64 B     Guest → Host flags (doorbell suppression, etc.)
0x00C0   64 B     Host → Guest flags (interrupt suppression, etc.)
0x0100   32 KiB   Submission Queue entries (512 × 64-byte SQEs)
0x8100   32 KiB   Completion Queue entries (512 × 64-byte CQEs)
0x10100  ~1.9 MiB Pre-registered data buffers (for fixed-buffer payload I/O)
```

### Control Block

```rust
#[repr(C, align(64))]
struct RingControl {
    // Submission queue (guest writes tail, linuxd writes head)
    sq_head: AtomicU32,       // consumer (linuxd) position
    sq_tail: AtomicU32,       // producer (guest) position
    sq_mask: u32,             // ring size - 1 (power of 2)
    sq_flags: AtomicU32,      // NEED_WAKEUP, etc.

    // Completion queue (linuxd writes tail, guest writes head)
    cq_head: AtomicU32,       // consumer (guest) position
    cq_tail: AtomicU32,       // producer (linuxd) position
    cq_mask: u32,
    cq_flags: AtomicU32,      // NOTIFY_GUEST, etc.
}
```

### Submission Queue Entry (64 bytes)

```rust
#[repr(C)]
struct SQEntry {
    opcode: u16,              // WRITE, READ, OPEN, CLOSE, STAT, ...
    flags: u16,               // LINKED, DRAIN, FIXED_BUF, ...
    fd: i32,                  // file descriptor
    user_data: u64,           // opaque tag returned in CQE (for async matching)
    addr: u64,                // buffer GPA (in data region) or inline offset
    len: u32,                 // byte count
    offset: u64,              // file offset (for pread/pwrite)
    _reserved: [u8; 26],
}
```

### Completion Queue Entry (64 bytes)

```rust
#[repr(C)]
struct CQEntry {
    user_data: u64,           // matches SQE.user_data
    result: i64,              // return value (bytes written, errno, etc.)
    flags: u32,               // MORE (more CQEs coming), BUFFER_ID, ...
    _reserved: [u8; 44],
}
```

---

## Doorbell Suppression (Key CPU Optimization)

This section describes the active notification-suppression scheme. Today the
host-to-guest side is implemented, and the direct-linuxd path also suppresses
guest-to-host doorbells while its bounded SQ polling window is open.

### Guest → Host: Suppress Doorbell

```rust
// Guest kernel, after posting SQE:
fn notify_host(ctrl: &RingControl) {
    sq_tail.store(new_tail, Release);

    // Only ring doorbell if linuxd asked for it.
    if ctrl.sq_flags.load(Acquire) & SQ_NEED_WAKEUP != 0 {
        out32(DOORBELL_PORT, 1);  // ioeventfd → wake linuxd
    }
    // Otherwise linuxd is already polling — no VM exit needed.
}
```

```rust
// linuxd, when entering polling window:
ctrl.sq_flags.fetch_and(!SQ_NEED_WAKEUP, Release);
// ... poll loop ...
// When parking on epoll:
ctrl.sq_flags.fetch_or(SQ_NEED_WAKEUP, Release);
```

### Host → Guest: Suppress Interrupt

```rust
// linuxd, after posting CQE:
fn notify_guest(ctrl: &RingControl, kvm_fd: &KvmFd) {
    cq_tail.store(new_tail, Release);

    // Only inject interrupt if guest is sleeping (HLT).
    if ctrl.cq_flags.load(Acquire) & CQ_NOTIFY_ME != 0 {
        kvm_fd.inject_interrupt(SYSCALL_VECTOR);
    }
    // Otherwise guest is polling CQ — no interrupt needed.
}
```

**Current effect**: CQ interrupts are suppressed when the guest is already
polling, and the direct-linuxd SQ worker suppresses guest doorbells while it is
still polling for follow-on SQEs. Not yet implemented: the Tier 3 dedicated
poll thread and the zero-notification steady state described by the original
design.

---

## Payload Data Path (Previous and Current)

The current tree contains two ring payload variants.

### Previous ring payload design (bulk compatibility path)

When `FIXED_BUF` is not set, the ring carries only metadata and the legacy host
path reconstructs a `DataChunk`:

```rust
guest kernel:
    post SqeOpcode::BulkData
    sqe.flags does not include FIXED_BUF

uservm ring_drain:
    rebuild DataChunk
    forward IkcFrame::Bulk

linuxd:
    receive owned payload bytes on the existing channel
```

This was the earlier ring payload design. It still exists as the compatibility
path for transfers that do not use fixed buffers.

### Current fixed-buffer payload design

When `FIXED_BUF` is set, the ring carries a fixed-buffer id and length instead
of bouncing the payload through the bulk path. The shared ring region is carved
into fixed-size 4 KiB slots that are mapped by both `uservm` and `linuxd`.

```rust
// Guest kernel write/send path.
let buf_id = data_buf_alloc();
copy_from_user(shared_fixed_buffer(buf_id), user_buf, len); // 1 copy
sqe.addr = buf_id as u64;
sqe.flags |= FIXED_BUF;
```

```rust
// linuxd host path.
let host_ptr = shared_ring.fixed_buffer_ptr(buf_id)?;
libc::pwrite(fd, host_ptr, len, offset); // host reads shared bytes directly
libc::pread(fd, host_ptr, len, offset);  // host writes shared bytes directly
```

Payload-copy summary for the implemented fixed-buffer path:

- `pwrite()` / send path: 1 guest copy from user memory into the shared fixed
  buffer; no extra host bounce buffer.
- `pread()` / receive path: the host fills the shared fixed buffer directly,
  then the guest copies once from that buffer back into user memory during CQ
  completion.
- This removes the old host-side bounce buffer on the benchmarked fixed-buffer
  path, but it is not yet a full end-to-end zero-copy receive path.

### Small payloads (planned, not yet implemented): Inline in SQE

For small writes (most IPC, control messages), the original design intended to
embed data directly in the SQE's inline space. The `INLINE` flag and storage
exist in `SqEntry`, but the active drain path does not use them yet.

### Large payloads: Fixed buffer region

The ~1.9 MiB data buffer region is carved into fixed-size slots (e.g.,
4 KiB each = 475 slots). The guest kernel manages a simple bitmap allocator:

```rust
// Guest kernel:
let buf_id: u16 = data_buf_alloc();           // get free slot
let buf_ptr: *mut u8 = data_buf_ptr(buf_id);  // GPA in shared region
copy_from_user(buf_ptr, user_buf, len);        // 1 copy: user → shared

sqe.addr = buf_id as u64;
sqe.flags |= FIXED_BUF;
```

```rust
// linuxd (zero-copy read):
let host_ptr: *const u8 = shared_region_base + buf_offset(sqe.addr);
libc::write(fd, host_ptr, sqe.len);  // host write() reads directly
                                      // from guest-mapped page
```

On the write/send path, the implemented fixed-buffer design reaches the 1-copy
goal shown above. On the read/receive path there is still one guest copy back
into the user buffer, so this is not yet a full end-to-end zero-copy design.

Direct per-syscall SQE opcodes (`Write`, `Read`, `Open`, `Close`, `Stat`, ...)
also remain future work in the active drain path; today it still handles `Nop`,
`IkcMessage`, and `BulkData`.

---

## Multi-page transfers

The current ring path now supports one logical `read()` / `write()` /
`pread()` / `pwrite()` transfer spanning multiple fixed buffers while keeping
the direct-`linuxd` hot path. The underlying reason this works is not that
guest user memory must be physically contiguous: the kernel already has
`vmcopy_from_user()` / `vmcopy_to_user()` helpers that can walk a process'
address space and copy across non-contiguous user pages. The transport limit is
now the configured logical transfer cap rather than a single 4 KiB buffer.

### Implemented design

1. **Cap each logical transfer** to a bounded size and split only above that
    cap. The initial target is **64 KiB per transfer** = `16 x 4 KiB` fixed
    buffers.
2. **Allocate multiple fixed buffers per request** in the guest kernel instead
   of a single per-thread buffer.
3. **Use `vmcopy_from_user()` / `vmcopy_to_user()`** to gather/scatter between
   the caller's user buffer and the ring-backed fixed-buffer region. This lets a
   single logical transfer span arbitrary guest user pages without depending on
   their physical contiguity.
4. **Keep one control request** (`PositionedWriteRequest` /
   `PositionedReadRequest`) but follow it with multiple ordered `FIXED_BUF`
   descriptors that together cover the requested byte range.
 5. **Have `linuxd` issue one host vectored syscall** for the whole logical
    request:
    - `write()` path: build a host `iovec[]` that points at the shared fixed
      buffers and call `writev()`.
    - `read()` path: build the same `iovec[]` and call `readv()`.
    - `pwrite()` path: build a host `iovec[]` that points at the shared fixed
      buffers and call `pwritev()`.
    - `pread()` path: build the same `iovec[]` and call `preadv()`.
6. **Batch receive-side completions while preserving guest-visible ownership semantics**:
    - For writes, keep the normal `WriteResponse` message as the visible
      completion.
    - For reads, let `linuxd` complete one host `readv()` / `preadv()` across
      the full ordered fixed-buffer list, then post one logical fixed-buffer
      completion carrying the total transferred length instead of one CQE per
      segment.
    - On the direct ring path this completion is encoded as
      `CqeFlags::BUFFER | CqeFlags::BATCH`; on the framed fallback path the
      `FixedBufferTransfer` carries `COMPLETION_BATCH`.
    - The guest still copies bytes from the shared fixed buffers back into the
      caller's user buffer in segment order and wakes the blocked `pull()`
      caller only once after the whole logical transfer completes. The final
      copy is still required to preserve user-buffer ownership semantics.
7. **Keep legacy guest binaries safe by feature-gating the larger chunk size**
   in the syscall crate. Ring-enabled guest builds raise the per-request chunk
   limit to `64 KiB`; legacy guest builds continue to split at page
   boundaries.

The trade-off is that the protocol now carries a bit more completion metadata
and the guest receive path must retain the segment list until the final
completion arrives, but the hot path removes per-segment CQ writes, guest CQ
polls, state lookups, and wakeups.

### Transfer-size rule of thumb

The cap should be large enough to amortize control-path costs, but small enough
that one request does not monopolize the fixed-buffer pool.

- Current ring region: `471` fixed buffers of `4096` bytes each.
- Recommended initial cap: `16` buffers = `64 KiB`.
- Share of pool consumed by one max-sized transfer: about `3.4%`.
- Concurrent max-sized transfers still possible per VM: about `29`.

That is a good starting point because it materially reduces per-page control
overhead without turning the fixed-buffer region into effectively dedicated
per-thread storage. If benchmarking later shows a clear benefit, the next
tuning step would be `128 KiB`, but `64 KiB` is the safer initial default.

### Guest API rollout

`pwrite()` / `pread()` were moved to this multi-buffer transport first, and the
same `64 KiB` ring-enabled chunking now also covers `write()` / `read()`.
Guest `pwritev()` / `preadv()` still layer on top of those improved paths. A
later optimization can teach guest vectored I/O to submit one logical
multi-iovec request directly instead of looping per `iovec`.

---

## Original Target Comparison (for Comparison Only)

The table below is the original end-state estimate that motivated the design.
It is not a summary of current measured behavior.

| Metric                    | Current (PIO + socket) | Ring Buffer (Tier 1) | Ring Buffer (Tier 2) |
|---------------------------|------------------------|----------------------|----------------------|
| Host wakeups / exits      | 3+                     | 1 doorbell wakeup    | 0                    |
| Data copies               | 3-4                    | 1                    | 1                    |
| Idle CPU                  | 0                      | 0                    | 0                    |
| Batching                  | No                     | Yes (drain loop)     | Yes                  |
| Latency (estimated)       | ~20-50 μs              | ~5-10 μs             | ~1-3 μs              |
| Throughput                | ~50K ops/s             | ~200K ops/s          | ~1M+ ops/s           |

---

## Implementation Phases

### Phase 1: Shared memory + eventfd doorbell (implemented in hybrid form)

1. Reserve 2 MiB region at a fixed GPA in the guest physical memory map.
2. In the VMM (`uservm`): create a KVM ioeventfd for the doorbell PIO port.
3. In `uservm`: create and map a shared ring backing file into guest memory;
   in `linuxd`: open the same backing file when fixed buffers are enabled.
4. Implement SPSC ring in a shared `nvx-ring` crate (used by both guest
   kernel and host-side components).
5. Guest kernel: new `send_sqe()` / `poll_cqe()` API replacing vmbus
   `write()` / `read()`.
6. Replace the `uservm` ring-drain thread with shared-memory signaling from
   `uservm` to `linuxd`, and let `linuxd` drain SQEs directly.
7. Write hot-path CQEs directly from `linuxd`, while keeping old socket/bulk
   handling as compatibility fallback.

### Phase 2: Adaptive polling (implemented as bounded SQ polling + CQ suppression)

8. Add spin loop to the drain loop with configurable idle timeout. Implemented
   on the direct-linuxd SQ worker.
9. Add doorbell suppression flags. Implemented on the direct-linuxd path with a
   bounded SQ polling window that clears/re-arms `SQ_NEED_WAKEUP`.
10. Add interrupt suppression flags + CQ polling in guest kernel. Implemented.

### Phase 3: Fixed-buffer data path (implemented)

11. Implement pre-registered buffer allocator in guest kernel.
12. Map the same shared ring backing into `linuxd`.
13. Add `FIXED_BUF` handling to the SQE / `IkcFrame::Fixed` path.

### Phase 4: Full syscall coverage (not yet implemented)

14. Port remaining syscalls (read, open, close, stat, mmap, ...) to SQE
    opcodes.
15. Deprecate old vmbus path.

---

## Current Status and Measured Results (2026-03-09 / 2026-03-11)

Detailed methodology and the full result tables live in `doc/benchmark.md`.

- The active ring path is now split into a direct hot path plus a compatibility
  fallback:
  - `linuxd` directly drains `IkcMessage` SQEs and `BulkData` SQEs marked with
    `FIXED_BUF`.
  - `linuxd` directly writes CQEs for regular `Message` responses and batched
    fixed-buffer read completions.
  - `uservm` still owns the KVM-facing doorbell eventfd and guest IRQ
    injection, translating both through shared notification words.
  - owned `IkcFrame::Bulk` compatibility responses still use the older framed
    socket path.
- The ring region is backed by a temp-file-backed `MAP_SHARED` file that
  `uservm` maps into guest memory and `linuxd` opens separately for fixed-buffer
  access.
- Historical fixed-size RTT data for the older Tier 1 / hybrid path that drained
  SQEs in `uservm` is retained below for comparison.
- CQ interrupt suppression is now implemented end-to-end.
- The previous `IkcFrame::Bulk` payload design remains in-tree as the
  compatibility path when `FIXED_BUF` is not used.
- The benchmarked `write()` / `read()` / `pwrite()` / `pread()` path now uses
  the fixed-buffer multi-page design: the ring shared region carries
  pre-registered payload buffers and the active path uses fixed-buffer
  descriptors instead of bouncing payload bytes through the older bulk path.
- The receive side now batches each logical fixed-buffer `read()` / `pread()`
  completion into one CQ event after the host `readv()` / `preadv()` finishes.
  The guest still performs the final fixed-buffer-to-user copy, but it now
  walks the stored segment list locally and wakes the blocked caller once.
- The fixed-buffer path is runtime-validated end-to-end, and the canonical
  `/dev/zero` payload sweep has been rerun against fresh legacy and ring
  artifact trees.
- The stronger direct path has now been benchmarked with:
  - a fresh 5-trial interleaved `fcntl(F_GETFL)` round-trip rerun with warm-up
    and pinned-core placement, and
  - a fresh 3-trial `/dev/zero` payload sweep that exercises fixed-buffer
    `write()` / `read()` / `pwrite()` / `pread()` traffic.
- Because the benchmark was rerun in a shared development environment, the
  absolute RTT numbers vary between historical runs; the interleaved medians and
  ring/legacy ratios are the more stable signal.
- Not yet implemented: Tier 3 dedicated polling, inline SQE payloads, direct
  handling of the `Write`/`Read`/`Open`/`Close`/`Stat` SQE opcodes, and full
  elimination of the socket fallback for compatibility `IkcFrame::Bulk`
  responses.

### Fixed-Size RTT

Using `fcntl(F_GETFL)` as a linuxd-backed round trip:

- Before CQ interrupt suppression:
  - legacy median = `454885 ns`
  - ring median = `621284 ns`
  - ring / legacy = `1.366x`
- After CQ interrupt suppression:
  - legacy median = `391014 ns`
  - ring median = `424155 ns`
  - ring / legacy = `1.085x`
- After direct linuxd SQ/CQ bypass:
  - legacy median = `241275 ns`
  - ring median = `118293 ns`
  - ring / legacy = `0.490x`

### Payload Sweeps

The payload sweeps now cover both sequential and positioned traffic (`write()`, `read()`,
`pwrite()`, and `pread()`) from `32` bytes up to `65536` bytes, including sizes well beyond a
single `4096`-byte page. The canonical payload results use `/dev/zero` as the linuxd-side backend
so that the benchmark reflects syscall/transport delay rather than host filesystem work.

Selected ring / legacy ratios from the current 3-trial median rerun:

| Operation | 4096 B | 8192 B | 16384 B | 32768 B | 65536 B |
|-----------|--------|--------|---------|---------|---------|
| `write()` | `0.290x` | `0.196x` | `0.130x` | `0.079x` | `0.060x` |
| `read()` | `0.244x` | `0.199x` | `0.136x` | `0.068x` | `0.046x` |
| `pwrite()` | `0.245x` | `0.234x` | `0.132x` | `0.078x` | `0.057x` |
| `pread()` | `0.302x` | `0.216x` | `0.186x` | `0.190x` | `0.135x` |

Interpretation:

- With `/dev/zero` backing the payload path, the direct-linuxd ring path now beats legacy at every
  measured size for all four operations.
- The strongest gains are still on the send side: at `65536` bytes, `write()` drops from
  `10.157 ms` to `0.613 ms`, and `pwrite()` drops from `10.424 ms` to `0.592 ms`.
- The new batched read-completion protocol materially improves the receive side too by removing the
  old per-segment CQ/control overhead while keeping the unavoidable final copy back into user
  space. At `65536` bytes, `read()` drops from `11.944 ms` to `0.551 ms`, and `pread()` drops from
  `9.983 ms` to `1.346 ms`.
- The fixed-size RTT rerun also now shows a cleaner absolute result after warm-up and core pinning:
  `0.241 ms` legacy vs `0.118 ms` ring for `fcntl(F_GETFL)`.
- These gains are consistent with removing the `uservm` SQ-drain / CQ-write hot path from the
  active transport path, amortizing one logical transfer across up to `16` shared fixed buffers,
  and collapsing receive-side completion traffic to one logical CQ event per `readv()` / `preadv()`
  result. The published numbers still predate the later bounded guest-to-host SQ polling window,
  and adaptive polling, guest-to-host doorbell suppression, and full fallback elimination are
  still pending.

## Key Design Decisions

**Q: Why ioeventfd instead of MMIO trap?**
KVM ioeventfd signals an eventfd in-kernel without exiting to the VMM
userspace process. An MMIO trap causes a full KVM_EXIT_MMIO to the VMM,
which then has to forward work through the rest of the host path. ioeventfd
lets the dedicated ring-drain path wake without that MMIO trap.

**Q: Why not virtio?**
Virtio is a great abstraction but carries significant complexity (virtqueues,
descriptor chains, feature negotiation). A custom SPSC ring is simpler,
tailored to our IPC model, and avoids the virtio driver stack.

**Q: What about multiple guests?**
Each guest gets its own shared ring backing file and doorbell eventfd inside
its `uservm` instance. `linuxd` still sees one existing stream per guest VM and,
when fixed buffers are enabled, one shared-ring mapping per VM.

**Q: Memory ordering?**
SPSC rings need only Release stores (producer) and Acquire loads (consumer).
No CAS, no locks. On x86_64, Release/Acquire map to plain stores/loads +
compiler fence — no MFENCE needed.
