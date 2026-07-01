# Design Document: Host Directory Mounting on the Nanvix Guest

This document describes the design of the host directory mounting feature that allows users to mount
a host directory into a Nanvix guest VM.

## 1. Problem Statement

Users need the ability to mount a host directory into a Nanvix guest VM so that guest applications
can read and write files from the host filesystem. Today, the only way to provide files to a guest
is via a pre-built RAMFS image passed with the `-ramfs` CLI flag. There is no mechanism to:

- Dynamically make a host directory available to the guest at launch time.
- Support multiple filesystem images simultaneously (e.g., a root RAMFS and a host-mounted
  directory).
- Synchronize guest-side modifications back to the host.

### User Interface

The feature is exposed through a `-mount <host-dir>` CLI flag for `nanvixd`.  The host directory
contents appear under `/mnt` inside the guest:

```shell
nanvixd -kernel K -initrd I -mount /path/to/host/dir -- guest.elf
```

Guest applications access host files through the standard VFS path (`/mnt/...`) using ordinary file
operations.

---

## 2. Requirements

### Goals

- Provide transparent access to a host directory from inside the Nanvix guest.
- Integrate with the existing VFS mount infrastructure inside the guest.
- Maintain backward compatibility: when `-mount` is absent, behavior is unchanged.

### Non-Goals

- Replacing `linuxd`. The mount feature covers only filesystem access to a specific host directory,
  not the full POSIX syscall surface.
- Supporting network-remote filesystems (NFS, SMB). The host directory is always local to the
  `nanvixd` host.
- POSIX permission or ownership fidelity. The guest operates under a single effective user;
  host-side permissions are those of the `nanvixd` process.

---

## 3. Design — Live Host Filesystem Forwarding (hostfsd)

The host directory is accessed live through the `hostfsd` daemon using IKC (Inter-Kernel
Communication). Guest file operations on `/mnt` are forwarded in real-time to the host process,
which performs the actual I/O on the host filesystem. There is no boot-time image packaging or
shutdown-time extraction step.

### 3.1 High-Level Design

```text
┌─ Launch ─────────────────────────────────────────────────────────┐
│                                                                  │
│  nanvixd -mount <host-dir> -- <guest.initrd>                     │
│       │                                                          │
│       ├─► UserVM spawns with hostfsd worker thread               │
│       │     (worker holds a reference to <host-dir>)             │
│       │                                                          │
│       └─► Guest boots, vfsd waits for explicit mount() syscall   │
│                                                                  │
├─ Runtime ────────────────────────────────────────────────────────┤
│                                                                  │
│  Guest app calls: mount("", "/mnt", "hostfs", 0)                 │
│       │                                                          │
│       ▼                                                          │
│  vfsd enables hostfs path routing for /mnt                       │
│       │                                                          │
│       ▼                                                          │
│  Guest file ops on /mnt ──► vfsd ──► IKC ──► hostfsd worker      │
│                                                ──► host std::fs  │
│                                                                  │
├─ Shutdown ───────────────────────────────────────────────────────┤
│                                                                  │
│  Guest calls: umount("/mnt")                                     │
│  (or VM exits — no extraction needed, host dir already updated)  │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 Components

| Component                       | Location                                          | Role                                                                                  |
| ------------------------------- | ------------------------------------------------- | ------------------------------------------------------------------------------------- |
| `mount()` / `umount()` syscalls | `src/libs/syscall/src/sys/mount/`                 | User-space API for mounting/unmounting hostfs                                         |
| vfsd mount handler              | `src/daemons/vfsd/src/handler/mount_handler.rs`   | Validates mount requests and enables/disables hostfs routing                          |
| vfsd hostfs module              | `src/daemons/vfsd/src/hostfs.rs`                  | Routes `/mnt` paths to IKC messages for hostfsd                                       |
| vfsd hostfs handlers            | `src/daemons/vfsd/src/handler/hostfs_handlers.rs` | Intercepts FD-based operations on hostfs-backed descriptors and forwards them via IKC |
| hostfs-api wire format          | `src/libs/hostfs-api/`                            | Defines the binary protocol between vfsd and hostfsd (request/response encoding)      |
| hostfsd daemon                  | `src/daemons/hostfsd/`                            | Host-side daemon that processes IKC requests using the host filesystem                |
| hostfsd worker                  | `src/uservm/src/standalone.rs`                    | Host-side thread that runs the hostfsd event loop                                     |

### 3.3 Architecture

#### Wire Protocol (`hostfs-api`)

All communication between vfsd (guest) and hostfsd (host) uses the `hostfs-api` wire format
encoded into the fixed-size IPC message payload. Each message carries:

- A 2-byte `SystemCallMessageHeader` discriminant identifying the operation (request or response).
- A 4-byte operation identifier (`op_id`) assigned by vfsd and echoed by hostfsd to correlate
  asynchronous responses with pending operations.
- 42 bytes of operation-specific data.

#### Asynchronous Request/Response Model

vfsd uses a non-blocking send model:

1. vfsd encodes the request, assigns an `op_id`, and sends an IKC message to hostfsd.
2. A `PendingOp` record is pushed onto a pending queue keyed by `op_id`.
3. vfsd continues processing other events.
4. When the IKC response arrives in the main event loop, vfsd matches it to the pending operation
   via `op_id` and completes the original guest syscall.

#### Path Routing

vfsd routes file operations to hostfsd based on path prefix:

- Paths matching `/mnt` or `/mnt/...` are forwarded to hostfsd when hostfs is enabled.
- FD-based operations (read, write, close, seek, stat, truncate, flush) check whether the FD
  is hostfs-backed and forward accordingly; non-hostfs FDs delegate to the standard FAT32 handlers.

#### Security — Path Sandbox

hostfsd constrains all guest-requested paths to a configured root directory (the host directory
passed via `-mount`). The sandbox:

- Canonicalizes paths to resolve `.` and `..` components.
- Rejects any resolved path that escapes the root directory (path traversal protection).
- Rejects symlinks that point outside the root.

#### File Descriptor Management

hostfsd maintains a remote file descriptor table that maps guest-visible FDs to host-side file
handles. The table:

- Allocates integer FDs starting at 1 (0 is reserved).
- Enforces a maximum number of simultaneously open descriptors per guest.
- Tracks whether each FD refers to a file or directory.
- Caches directory listings for readdir operations (populated on first access).
- Invalidates directory caches when mutating operations (mkdir, rmdir, unlink, rename) occur.

### 3.4 Mount Lifecycle

1. **Guest boot:** vfsd initializes but does NOT enable hostfs forwarding.
2. **Explicit mount:** Guest application calls `mount("", "/mnt", "hostfs", 0)` which sends an
   IPC message to vfsd. The mount handler validates the request (only "hostfs" type at "/mnt" is
   accepted) and enables the hostfs path routing.
3. **File operations:** Any VFS operation on paths under `/mnt` is intercepted by vfsd and
   forwarded via IKC to the hostfsd worker on the host, which performs the operation on the actual
   host directory.
4. **Unmount:** Guest calls `umount("/mnt")` to disable hostfs forwarding. All changes are
   already persisted on the host filesystem (no extraction needed).

### 3.5 Advantages Over Snapshot-Based Approach

| Property           | hostfsd (current)                  | Snapshot-based (removed)         |
| ------------------ | ---------------------------------- | -------------------------------- |
| Boot latency       | Constant (no image packing)        | O(n) proportional to dir size    |
| Size limit         | Unbounded (host filesystem)        | Limited by guest memory (16 MiB) |
| Host→guest sync    | Live (reads see latest host state) | Stale after boot                 |
| Guest→host sync    | Immediate (writes go to host)      | Only on shutdown                 |
| Guest memory usage | Zero (no in-memory copy)           | Full directory size              |

### 3.6 Supported Operations

The following file operations are supported on hostfs-mounted paths (`/mnt/...`):

| Operation      | Syscall                  | Notes                                                                                      |
| -------------- | ------------------------ | ------------------------------------------------------------------------------------------ |
| Open/Create    | `openat()`               | Supports `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_CREAT`, `O_TRUNC`, `O_DIRECTORY`, `O_APPEND` |
| Close          | `close()`                | Releases local and remote FD                                                               |
| Read           | `read()` / `pread()`     | Positional reads supported; data clamped per IKC round-trip; caller handles short reads    |
| Write          | `write()` / `pwrite()`   | Positional writes supported; data clamped per IKC round-trip; caller handles short writes  |
| Seek           | `lseek()`                | `SEEK_SET`, `SEEK_CUR`, `SEEK_END`                                                         |
| Stat           | `fstat()`                | Returns size, mode (POSIX permissions on Unix, synthetic on Windows), and type             |
| Truncate       | `ftruncate()`            | Truncates to specified length; rejected on directories                                     |
| Sync           | `fsync()`                | Flushes host-side file buffers                                                             |
| Read Directory | `getdents()`             | Offset-based iteration via cached directory listing                                        |
| Mkdir          | `mkdirat()`              | Creates directory on host                                                                  |
| Rmdir          | `unlinkat(AT_REMOVEDIR)` | Removes empty directory on host                                                            |
| Unlink         | `unlinkat()`             | Removes file on host                                                                       |
| Rename         | `renameat()`             | Both paths must resolve within the sandbox                                                 |

#### Not Supported

| Operation                   | Reason                                              |
| --------------------------- | --------------------------------------------------- |
| `fstatat()` on `/mnt` paths | Requires path-based stat without pre-opened FD      |
| `link()` / `symlink()`      | Not applicable to hostfs                            |
| `chmod()` / `fchmod()`      | Host permissions are those of the `nanvixd` process |
| `fallocate()`               | Not forwarded to hostfsd                            |

### 3.7 C Bindings

C-compatible bindings are provided for `mount()` and `umount()`:

```c
int mount(const char *source, const char *target, const char *fstype, unsigned long flags);
int umount(const char *target);
```

These are defined in `src/libs/syscall/src/sys/mount/bindings/` and follow the same pattern as
other syscall C bindings (validate pointers, convert to Rust strings, call safe implementation,
set errno on error, return -1 on failure / 0 on success).

### 3.8 Edge Cases

1. **No `-mount` flag** — No hostfsd worker is spawned. Guest `mount("hostfs")` still succeeds
   (vfsd enables path routing unconditionally), but subsequent file operations on `/mnt` paths will
   fail because the host-side handler is absent and the IKC channel has no receiver.
2. **Mount before calling `mount()`** — File operations on `/mnt` fail with an error since
   hostfs routing is not enabled.
3. **Double mount** — Rejected with `ResourceBusy` error.
4. **Umount without mount** — Rejected with `InvalidArgument` error.
5. **VM exit without umount** — No data loss; all writes were already committed to the host.
6. **Path traversal** — Paths resolving outside the sandbox root are rejected with a permission
   error.
7. **FD table exhaustion** — Opening more files than the maximum limit returns an I/O error.

---

## 4. Testing

### Integration Test: `mount-test`

The `mount-test` binary (`src/tests/integration/mount-test/`) is a comprehensive guest-side integration test
that exercises the full hostfs feature set in standalone mode. It is structured into phases:

1. **Mount lifecycle** — mount, double-mount rejection, umount, double-umount rejection, re-mount.
2. **Filesystem operations** — mkdir/rmdir, create/unlink, rename on `/mnt` paths.
3. **File operations** — write/read, seek (from start/end), fstat (size verification),
   ftruncate, fsync on `/mnt` paths.
4. **Cross-filesystem consistency** — performs identical operations on both RAMFS and hostfs,
   verifying that results and error codes are consistent.

The test is run via:

```bash
nanvixd -mount ./bin/mount-test-data -- mount-test.initrd
```

The `mount-test-data/` directory is reset to pristine state before each test run by the build
system.

### Test Configuration

The test entry in `test/test-standalone-windows.toml`:

```toml
[[tests]]
executor = "terminal"
program = "./bin/mount-test.initrd"
extra_nanvixd_args = "-ramfs ./bin/standalone-rootfs.img -mount ./bin/mount-test-data"
expected_output = "ok"
expected_exit_code = 0
runs_on = ["microvm"]
```
