# Design: MMIO-Based VMBus

This document proposes replacing the current PMIO-based VMBus with an
MMIO-backed shared-memory ring buffer. The goal is to reduce VM-exit overhead
on the guest-to-host communication path, which is the dominant cost under
nested AMD SVM virtualization.

## Motivation

The current VMBus uses two PMIO ports for message exchange:

- Port `0xe9` (stdout): the guest writes a guest-physical address via `out32`,
  triggering a `KVM_EXIT_IO`. The VMM reads the `Message` from guest memory.
- Port `0xea` (stdin): the guest writes a buffer address via `out32`, triggering
  a `KVM_EXIT_IO`. The VMM writes a `Message` into guest memory.

Every message therefore requires **one VM exit**, which costs ~5-15 us under
nested virtualization (L0 AMD hardware, L1 Hyper-V, L2 WSL2 KVM, L3 Nanvix).
The `echo-breakdown` benchmark shows that VMBus round-trips dominate end-to-end
latency.

## Current Architecture

```
Guest Kernel                              VMM (uservm)
+------------------+                      +------------------+
|                  |   out32(0xe9, addr)   |                  |
|  vmbus_write()  -|---[KVM_EXIT_IO]----->|  stdout_fn()     |
|                  |                       |  read_bytes()    |
|                  |                       |                  |
|                  |   out32(0xea, addr)   |                  |
|  vmbus_read()   -|---[KVM_EXIT_IO]----->|  stdin_fn()      |
|                  |                       |  write_bytes()   |
|                  |                       |                  |
|  credits reg    <|---[volatile read]-----|  add_credit()    |
+------------------+                      +------------------+
```

Key files:
- `src/kernel/src/hal/platform/microvm.rs` (`vmbus_write`, `vmbus_read`)
- `src/kernel/src/stdio.rs` (credits check, message dispatch)
- `src/uservm/src/vmm/microvm/emulator.rs` (`handle_pmio_access`)
- `src/uservm/src/vmm/microvm/guest.rs` (`add_credit`, `consume_credit`)
- `src/libs/config/src/lib.rs` (port numbers, control register offsets)

## Proposed Architecture

Replace per-message PMIO exits with a pair of shared-memory ring buffers that
both sides access directly. VM exits are reduced to an optional doorbell
notification when the guest has produced messages that need processing.

```
Guest Physical Memory:
+-----------------------+ CTRL_BASE (0x00000000)
| Control Registers     |   null, credits, pause, ramfs_base, ramfs_size,
|                       |   tx_ring_base, tx_ring_size,
|                       |   rx_ring_base, rx_ring_size,
|                       |   doorbell (write-only)
+-----------------------+ TX_RING_BASE
| TX Ring Buffer        |   Producer: guest kernel
| (N pages, e.g. 16 KB)|   Consumer: VMM
+-----------------------+ RX_RING_BASE
| RX Ring Buffer        |   Producer: VMM
| (N pages, e.g. 16 KB)|   Consumer: guest kernel
+-----------------------+
| Ring Headers (TX, RX) |   head, tail, capacity, flags
+-----------------------+
```

### Ring Buffer Header

Each ring has a header in a shared memory region, accessed via volatile
reads/writes:

```rust
#[repr(C)]
struct RingHeader {
    head: u32,      // Written by producer, read by consumer.
    tail: u32,      // Written by consumer, read by producer.
    capacity: u32,  // Number of message slots.
    flags: u32,     // Doorbell pending, overflow, etc.
}
```

Each slot holds one fixed-size `Message` (as defined by `IPC_MESSAGE_SIZE`).
Slot `i` is at offset `ring_base + i * size_of::<Message>()`.

### TX Path (Guest to Host)

1. Compute free space: `free = capacity - (head - tail)`.
2. Copy the `Message` into `tx_buffer[head % capacity]` (volatile write).
3. Increment `head` (volatile write).
4. Optionally write to the **doorbell** MMIO address to notify the VMM.

Without the doorbell, **zero VM exits** occur per message. The VMM discovers
new messages by polling the TX ring head index from the host-mapped guest
memory.

### RX Path (Host to Guest)

1. VMM writes the `Message` into `rx_buffer[head % capacity]` via the
   host-mapped guest memory pointer and increments `head`.
2. Guest reads `rx_ring.head` (volatile) to check availability.
3. Guest copies the message from `rx_buffer[tail % capacity]` and increments
   `tail`.

No VM exit is needed. The guest polls the head index. If the guest is blocked
waiting for a message, the VMM can inject an interrupt to wake it.

### Doorbell Mechanism

The doorbell is a single MMIO address in the control register page. A guest
write to this address triggers `KVM_EXIT_MMIO`, which the VMM uses as a
signal to drain the TX ring. This amortizes the exit cost across multiple
queued messages.

The doorbell is **optional**. In latency-sensitive workloads where the VMM
polls, it can be skipped entirely. In throughput-oriented workloads, the guest
can batch several messages and ring the doorbell once.

### VMM Consumption Strategies

| Strategy     | Mechanism                                         | Trade-off                    |
|--------------|---------------------------------------------------|------------------------------|
| Polling      | VMM busy-loops on `tx_ring.head != tx_ring.tail`  | Lowest latency, uses CPU     |
| Doorbell     | VMM blocks on `KVM_EXIT_MMIO`                     | Low CPU, higher latency      |
| Hybrid       | Poll for N us, then fall back to doorbell wait    | Balanced latency and CPU     |

### Memory Ordering

x86 provides Total Store Order (TSO): stores are visible in program order.
For a single-producer, single-consumer ring buffer, volatile reads/writes are
sufficient. No explicit memory barriers are needed.

### Performance Comparison

| Metric                  | PMIO (current)                | MMIO Ring Buffer                   |
|-------------------------|-------------------------------|------------------------------------|
| VM exits per TX message | 1 (`out32` -> `KVM_EXIT_IO`) | 0 (polling) or amortized (doorbell)|
| VM exits per RX message | 1 (`out32` -> `KVM_EXIT_IO`) | 0 (volatile read only)            |
| Data copying            | VMM reads guest mem after exit| Direct ring buffer access          |
| Batching                | Impossible (1 exit = 1 msg)  | Natural (N messages, 0-1 exits)   |
| Credits mechanism       | Volatile read + PMIO exit    | Replaced by ring free-space check  |

## Implementation Plan

### Step 1: Extend Control Register Layout

Add the following offsets to `config::microvm` in `src/libs/config/src/lib.rs`:

| Register           | Offset   | Description                              |
|--------------------|----------|------------------------------------------|
| `CTRL_TX_RING_BASE`| `0x0014` | Guest-physical base of the TX ring.      |
| `CTRL_TX_RING_SIZE`| `0x0018` | Size of the TX ring in bytes.            |
| `CTRL_RX_RING_BASE`| `0x001c` | Guest-physical base of the RX ring.      |
| `CTRL_RX_RING_SIZE`| `0x0020` | Size of the RX ring in bytes.            |
| `CTRL_DOORBELL`    | `0x0024` | Write-only doorbell (triggers MMIO exit).|

The VMM writes these values during guest setup, alongside the existing
`CTRL_RAMFS_BASE` and `CTRL_RAMFS_SIZE`.

### Step 2: Allocate Ring Buffers (VMM Side)

In `src/uservm/src/vmm/microvm/guest.rs`:

- Allocate contiguous guest-physical pages for the TX and RX rings (e.g., 4
  pages each = 16 KB, holding ~100 messages per ring at current message size).
- Initialize ring headers with `head = 0`, `tail = 0`,
  `capacity = ring_size / size_of::<Message>()`.
- Write the ring base addresses and sizes into the control register page.

### Step 3: Guest Ring Buffer Driver

Create `src/kernel/src/hal/platform/vmbus_ring.rs`:

- `init()`: read ring base/size from control registers via volatile reads. If
  the ring registers are zero, fall back to the existing PMIO path.
- `send(msg: &Message) -> Result<(), Error>`: write to the TX ring, optionally
  write to the doorbell address.
- `recv() -> Result<Option<Message>, Error>`: read from the RX ring.

Replace calls to `vmbus_write()` and `vmbus_read()` in
`src/kernel/src/hal/platform/microvm.rs` with the ring buffer functions.

### Step 4: VMM Ring Buffer Consumer

In `src/uservm/src/vmm/microvm/emulator.rs`:

- Add an MMIO handler for the doorbell address: on `KVM_EXIT_MMIO` write to
  the doorbell, drain the TX ring and forward messages to the Linux daemon
  channel.
- Replace `stdin_fn` with a function that writes incoming messages directly
  into the RX ring via the host-mapped guest memory pointer.
- Remove the credits-based flow control (ring free-space replaces it).

### Step 5: Fallback and Gradual Rollout

Keep the existing PMIO path as a compile-time or runtime fallback:

- If the guest detects `CTRL_TX_RING_BASE == 0` at boot, it uses the PMIO
  path unchanged.
- This allows benchmarking both paths side by side and ensures backward
  compatibility with older VMM versions.

## Design Decisions

### Fixed-Size Slots

Message slots use the existing fixed-size `Message` struct
(`IPC_MESSAGE_SIZE`). This simplifies ring indexing
(`base + index * MESSAGE_SIZE`) and avoids fragmentation. Variable-length
messages would require a more complex protocol with length headers and
wrap-around handling.

### Single-Producer, Single-Consumer

Each ring has exactly one producer and one consumer:
- TX ring: guest kernel (producer), VMM (consumer).
- RX ring: VMM (producer), guest kernel (consumer).

This avoids synchronization overhead. If multiple guest threads need to send
concurrently, the kernel serializes access internally (as it already does for
the PMIO path via `vmbus_write`).

### Doorbell vs. Interrupt

The doorbell (guest-to-host notification) uses an MMIO write because
`KVM_EXIT_MMIO` is cheaper than alternative signaling mechanisms. For
host-to-guest notification, an injected interrupt (e.g., a dedicated IRQ line)
wakes a blocked guest without polling. The interrupt path can be added
incrementally after the ring buffer is functional.

### Ring Size

A ring of 4 pages (16 KB) holds approximately 100 messages. This is large
enough to absorb bursts without backpressure in typical workloads. The size is
configurable via the control registers, so it can be tuned per deployment.
