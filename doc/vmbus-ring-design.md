# Shared-Memory Ring Buffer VMBus — Design Sketch

## Problem

Every guest syscall today costs 3+ KVM exits, 3–4 data copies, and a Unix
socket round-trip. Busy-polling (vhost-user/DPDK style) fixes latency but
wastes CPU cores.

## Goal

Near-native syscall throughput **without** dedicating host cores to polling.

---

## Architecture Overview

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

The key to avoiding dedicated cores is an **adaptive notification** strategy
with three tiers that the system switches between automatically:

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

**Cost**: 1 VM exit (doorbell) + 1 interrupt inject per batch.
**CPU when idle**: zero — both sides sleep on epoll/HLT.

KVM ioeventfd is the crucial primitive here: the guest writes to a PIO port,
and KVM signals an eventfd **in-kernel** without a full VM exit to userspace.
This is how virtio-pci doorbells work.

### Tier 2 — Adaptive Polling (auto-enabled under load)

When linuxd sees sustained SQ traffic, it enters a short polling window:

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

**Cost**: 0 VM exits while polling window is open.
**CPU when idle**: returns to zero after the polling window expires.
This is exactly how io_uring SQPOLL works — it polls for a configurable
idle period, then parks.

### Tier 3 — Dedicated Poll Thread (opt-in, maximum throughput)

For benchmarks or throughput-critical deployments, pin a host core to
poll the SQ continuously. Same as vhost-user. Enabled by flag, never default.

**Automatic tier selection:**

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
0x10100  ~1.9 MiB Pre-registered data buffers (for zero-copy)
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

The expensive operations are the doorbell (VM exit) and the interrupt inject.
Both can be suppressed when the other side is already awake:

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

**Net effect**: Under sustained load, **zero VM exits and zero interrupts**.
When idle, both sides sleep with zero CPU.

---

## Data Path: Zero-Copy via Pre-Registered Buffers

### Small payloads (≤ 256 bytes): Inline in SQE

For small writes (most IPC, control messages), embed data directly in the
SQE's reserved space. No separate buffer needed.

### Large payloads: Pre-Registered Buffer Region

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

**Total copies: 1** (user space → shared buffer). The host reads the same
physical page. Compare to 3-4 copies today.

---

## Comparison: Current vs Proposed

| Metric               | Current (PIO + socket) | Ring Buffer (Tier 1) | Ring Buffer (Tier 2) |
|-----------------------|------------------------|----------------------|----------------------|
| VM exits per syscall  | 3+                     | 1 (doorbell)         | 0                    |
| Data copies           | 3-4                    | 1                    | 1                    |
| Idle CPU              | 0                      | 0                    | 0                    |
| Batching              | No                     | Yes (drain loop)     | Yes                  |
| Latency (estimated)   | ~20-50 μs              | ~5-10 μs             | ~1-3 μs              |
| Throughput            | ~50K ops/s             | ~200K ops/s          | ~1M+ ops/s           |

---

## Implementation Phases

### Phase 1: Shared memory + eventfd doorbell (Tier 1)

1. Reserve 2 MiB region at a fixed GPA in the guest physical memory map.
2. In the VMM (uservm): create a KVM ioeventfd for the doorbell PIO port.
3. In linuxd: mmap the guest memory fd at the shared region offset.
4. Implement SPSC ring in a shared `nvx-ring` crate (used by both guest
   kernel and linuxd).
5. Guest kernel: new `send_sqe()` / `poll_cqe()` API replacing vmbus
   `write()` / `read()`.
6. linuxd: epoll loop that drains SQ, executes syscalls, posts CQEs.
7. Wire up `write()` as the first syscall on the new path. Keep old vmbus
   as fallback.

### Phase 2: Adaptive polling (Tier 2)

8. Add spin loop to linuxd drain loop with configurable idle timeout.
9. Add doorbell suppression flags.
10. Add interrupt suppression flags + CQ polling in guest kernel.

### Phase 3: Zero-copy data buffers

11. Implement pre-registered buffer allocator in guest kernel.
12. Map buffer region into linuxd.
13. Add FIXED_BUF flag to SQE path.

### Phase 4: Full syscall coverage

14. Port remaining syscalls (read, open, close, stat, mmap, ...) to SQE
    opcodes.
15. Deprecate old vmbus path.

---

## Current Status and Measured Results (2026-03-09)

Detailed methodology and the full result tables live in `doc/benchmark.md`.

- The fixed-size RTT data still reflects the Tier 1 / hybrid path:
  shared-memory SQ/CQ + ioeventfd doorbell + host drain thread + linuxd syscall handling.
- CQ interrupt suppression is now implemented end-to-end.
- Positioned `pwrite()` / `pread()` now use the fixed-buffer Phase 3 design: the ring shared region
  carries pre-registered payload buffers and the host transport forwards `IkcFrame::Fixed`
  descriptors instead of bouncing payload bytes through the older bulk push/pull path.
- The fixed-buffer path is now runtime-validated end-to-end, and the canonical `/dev/zero` payload
  sweep has been rerun against fresh legacy and ring artifact trees.
- Tier 2 adaptive polling is still pending.

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

### Payload Sweeps

The payload sweeps now cover both directions (`pwrite()` and `pread()`) from `32` bytes up to
`32768` bytes, including sizes beyond a single `4096`-byte page. The canonical payload results use
`/dev/zero` as the linuxd-side backend so that the benchmark reflects syscall/transport delay
rather than host filesystem work.

Selected ring / legacy ratios from the current 3-trial median rerun:

| Operation | 4096 B | 8192 B | 16384 B | 32768 B |
|-----------|--------|--------|---------|---------|
| `pwrite()` | `1.104x` | `1.057x` | `1.137x` | `1.010x` |
| `pread()` | `0.959x` | `0.921x` | `0.946x` | `1.002x` |

Interpretation:

- With `/dev/zero` backing the payload path, the absolute `32768`-byte latencies are now
  `4.320 ms` legacy vs `4.363 ms` ring for `pwrite()`, and `4.540 ms` legacy vs `4.550 ms` ring
  for `pread()`.
- Compared with the earlier bulk-path rerun, fixed buffers materially improved the large-payload
  behavior. `pread()` is now at or below legacy from `4096` B through `16384` B, and both
  directions are effectively at parity by `32768` B.
- `pwrite()` still carries a modest residual overhead on most points, which suggests that the
  remaining costs are concentrated in the shared host pipeline—drain-thread forwarding, linuxd
  processing, and CQ completion handling—rather than in the host storage backend.

## Key Design Decisions

**Q: Why ioeventfd instead of MMIO trap?**
KVM ioeventfd signals an eventfd in-kernel without exiting to the VMM
userspace process. An MMIO trap causes a full KVM_EXIT_MMIO to the VMM,
which then has to forward to linuxd. ioeventfd cuts out the middleman.

**Q: Why not virtio?**
Virtio is a great abstraction but carries significant complexity (virtqueues,
descriptor chains, feature negotiation). A custom SPSC ring is simpler,
tailored to our IPC model, and avoids the virtio driver stack.

**Q: What about multiple guests?**
Each guest gets its own shared memory region and doorbell. linuxd's epoll
loop handles multiple ioeventfds. The ring crate is instantiated per-guest.

**Q: Memory ordering?**
SPSC rings need only Release stores (producer) and Acquire loads (consumer).
No CAS, no locks. On x86_64, Release/Acquire map to plain stores/loads +
compiler fence — no MFENCE needed.
