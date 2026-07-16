# Deterministic Snapshotting in Standalone Mode

This document describes guidelines for taking and restoring deterministic snapshots in Nanvix
standalone build modes, with emphasis on interactions with RAMFS and host-mounted filesystems.

## 1. Overview

A snapshot captures the full execution state of a Nanvix guest VM at a specific point in time,
enabling fast warm-start restores that skip kernel boot, initrd loading, and daemon initialization.
Snapshots are supported by the standard Nanvix build.

A snapshot consists of two files stored in `snapshots/` relative to the working directory:

| Platform    | Memory file             | State file                  |
| ----------- | ----------------------- | --------------------------- |
| Linux/KVM   | `snapshots/<stem>.vmem` | `snapshots/<stem>.kvm.json` |
| Windows/WHP | `snapshots/<stem>.vmem` | `snapshots/<stem>.whp.cbor` |

Where `<stem>` is derived from the kernel filename (e.g., `kernel` from `bin/kernel.elf`).

## 2. What a Snapshot Captures

### 2.1 Deterministic State (Included)

| Component             | Storage      | Notes                                                                      |
| --------------------- | ------------ | -------------------------------------------------------------------------- |
| Guest physical memory | `.vmem` file | All pages, including kernel heap, stacks, and daemon data                  |
| vCPU registers        | State file   | RIP, RSP, general-purpose, segment, control registers                      |
| Interrupt controller  | State file   | LAPIC/IOAPIC state                                                         |
| Timer state           | State file   | PIT/LAPIC timer configuration                                              |
| RAMFS contents        | Guest memory | RAMFS is loaded into guest physical memory; any modifications are captured |
| vfsd state            | Guest memory | File descriptor table, pending queue, hostfs enabled flag                  |

### 2.2 Non-Deterministic State (Not Included)

| Component             | Reason                                                    |
| --------------------- | --------------------------------------------------------- |
| Host TSC value        | Host CPU timestamp counter continues independently        |
| Wall-clock time       | `pvclock boot_time_ns` is not re-written on restore       |
| IKC channels          | Host-side Tokio mpsc channels are recreated on each spawn |
| hostfsd worker thread | Host OS thread with open file descriptors; spawned fresh  |
| Host file descriptors | Kernel fd table entries are per-process, not serialized   |
| Network sockets       | Host kernel state; not captured                           |

## 3. Guidelines for Deterministic Snapshots

### 3.1 Rule: Snapshot Before Mounting HostFS

To guarantee deterministic restore, **take the snapshot before any host-mounted filesystem
operations**:

```text
boot → daemon init → [snapshot] → mount hostfs → workload
```

This ensures:

- `HOSTFS_ENABLED` is `false` in the snapshot — no hostfs routing occurs on restore until
  explicitly re-mounted.
- The vfsd pending queue is empty — no orphaned IKC operations waiting for responses that
  will never arrive.
- The fd table contains no hostfs-backed descriptors — no stale `remote_fd` references to
  non-existent host file handles.
- RAMFS state is fully self-contained in guest memory — byte-for-byte reproducible.

### 3.2 Rule: RAMFS State is Part of the Snapshot

RAMFS data resides entirely in guest physical memory. Any files created, modified, or deleted
on RAMFS before the snapshot are captured and restored exactly. This makes RAMFS the safe
filesystem for pre-populating guest state before snapshotting:

```text
boot → create files on RAMFS → [snapshot] → restore (files are present)
```

### 3.3 Rule: HostFS Reinitializes on Restore

On snapshot restore, the host-side `standalone_io_handler` spawns a fresh `hostfsd-worker`
thread. This worker has:

- No open file descriptors from the previous session.
- No knowledge of the guest's pre-snapshot hostfs state.
- A clean mapping to the `-mount` directory (if provided).

The guest must call `mount("", "/mnt", "hostfs", 0)` after restore to establish a new
hostfs session.

### 3.4 Rule: One Snapshot Per VM Lifetime

The kernel grants exactly one snapshot opportunity per boot, gated by the `snapshot` kernel
argument. After the snapshot is taken, subsequent `snapshot()` calls return an error.

## 4. Snapshot Workflow

### 4.1 Taking a Snapshot (Cold Boot Phase)

```text
nanvixd -snapshot bin/kernel.elf -- bin/snapshot-program.initrd
```

The guest program triggers the snapshot via:

```rust
::sys::kcall::pm::snapshot()?;
```

For C callers, the equivalent is:

```c
int ret = __kcall_snapshot();
assert(ret == 0);
```

This issues a kernel call that writes to I/O port `0x604` with the snapshot command. The VMM
intercepts the port-I/O exit, serializes guest memory and CPU state to disk, then allows the
guest to continue (or exit).

**Prerequisites:**

- The kernel must be booted with the `"snapshot"` kernel argument.
- The `snapshots/` directory must exist.
- Standalone build mode is required.

### 4.2 Restoring from a Snapshot (Warm Start)

```text
nanvixd -snapshot bin/kernel.elf -mount ./data -- bin/workload.initrd
```

When `-snapshot` points to an existing snapshot, the restore path:

1. Creates VM infrastructure (partition, virtual memory, vCPU).
2. **Skips** kernel, initrd, and RAMFS loading entirely.
3. Maps the `.vmem` file as guest physical memory (COW on WHP, `MAP_FIXED` on KVM).
4. Restores vCPU registers, interrupt controller, and timer state from the state file.
5. Sets `skip_next_snapshot = true` to prevent re-triggering.
6. Resumes execution from the instruction following the snapshot call.

The guest resumes as if `snapshot()` just returned `Ok(())`.

### 4.3 The `skip_next_snapshot` Mechanism

On KVM, the vCPU's RIP remains on the `out` instruction that triggered the snapshot (KVM
does not advance RIP for I/O exits). On restore, KVM re-executes that instruction, generating
another snapshot command exit. The `skip_next_snapshot` flag causes the VMM to silently skip
this re-triggered request.

On WHP, RIP is advanced past the `out` instruction before the exit is delivered, so
re-triggering does not occur. The guard is present defensively.

## 5. Interaction with Host-Mounted Filesystem

### 5.1 Safe Pattern: Snapshot Before Mount

```text
[Cold boot]
  kernel boots → procd/memd/vfsd initialize → snapshot()

[Warm restore]
  resume → mount("", "/mnt", "hostfs", 0) → open/read/write on /mnt → umount → exit
```

In this pattern:

- The snapshot contains only kernel + daemon initialization state.
- HostFS is mounted fresh after each restore with a clean hostfsd worker.
- Each restore is deterministic regardless of host directory contents at snapshot time.

### 5.2 Unsafe Pattern: Snapshot After Mount (Avoid)

```text
[Cold boot]
  kernel boots → mount hostfs → open files → snapshot()  ← UNSAFE

[Warm restore]
  resume → use stale FDs → UNDEFINED BEHAVIOR
```

Problems with this pattern:

- Guest FDs reference `remote_fd` values from the old hostfsd worker.
- The new hostfsd worker has no mapping for those remote FDs.
- Pending IKC operations in the queue will never receive responses.
- Callers block indefinitely or receive unexpected errors.

### 5.3 Future: Hostfs-Aware Snapshots

If hostfs-aware snapshots become necessary, the restore path would need to:

1. Call `PendingQueue::drain_with_error()` to fail all in-flight operations.
2. Invalidate all FDs marked as hostfs-backed (`is_hostfs_remote`).
3. Signal the guest to re-mount hostfs, re-establishing the session.

The `drain_with_error()` method exists in the codebase as recovery infrastructure but is
not yet wired into the restore path.

## 6. Warm-Start Benchmark Pattern

The canonical warm-start benchmark demonstrates the correct snapshot usage:

**Phase 1 — Create snapshot (once):**

```rust
UserVm::spawn(UserVmArgs {
    kernel_filename: "bin/kernel.elf",
    initrd_filename: Some("snapshot-rust-nostd.elf"),
    kernel_args: Some("snapshot"),  // grants one-shot permission
    snapshot_path: None,            // cold boot
    ramfs_filename: None,
    ..
});
// Guest calls snapshot(), VMM saves state, guest exits.
```

**Phase 2 — Measure restore latency (N iterations):**

```rust
let start = Instant::now();
UserVm::spawn(UserVmArgs {
    kernel_filename: "bin/kernel.elf",
    initrd_filename: None,                          // not needed
    kernel_args: None,                              // not needed
    snapshot_path: Some("bin/kernel.elf"),           // triggers restore
    mount_directory: Some("./data".to_string()),    // hostfs available post-restore
    ..
});
// Guest resumes, mounts hostfs, runs workload, exits.
latencies.push(start.elapsed());
```

## 7. Checklist for Deterministic Snapshots

- [ ] Build Nanvix.
- [ ] Pass `"snapshot"` as a kernel argument during the cold-boot phase.
- [ ] Ensure `snapshots/` directory exists before taking the snapshot.
- [ ] Take the snapshot **before** calling `mount("", "/mnt", "hostfs", 0)`.
- [ ] Do not hold open hostfs file descriptors at snapshot time.
- [ ] Ensure the vfsd pending queue is empty (no in-flight IKC operations).
- [ ] On restore, provide `-mount <dir>` if hostfs access is needed post-restore.
- [ ] On restore, the guest must explicitly mount hostfs again.
- [ ] RAMFS state from before the snapshot is available immediately on restore.
- [ ] Do not rely on wall-clock time being consistent across restores.
