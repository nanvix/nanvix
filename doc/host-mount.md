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

## 3. Design — Snapshot-Based Mounting

This is an initial, straightforward implementation that requires minimal changes to the existing
system architecture. It works by packaging the host directory into an in-memory disk image before
the guest boots and extracting modifications after the guest exits.

### 3.1 High-Level Design

```text
┌─ Launch ─────────────────────────────────────────────────────────┐
│                                                                  │
│  Host directory ──► FAT32 image ──► Multi-image container        │
│                     (mkramfs)       (ROOTFS + MOUNTFS)           │
│                                            │                     │
│                                            ▼                     │
│                                     Guest memory (MMIO)          │
│                                            │                     │
│                                            ▼                     │
│                                   Guest VFS mounts:              │
│                                     /     ← ROOTFS               │
│                                     /mnt  ← MOUNTFS              │
│                                                                  │
├─ Shutdown ───────────────────────────────────────────────────────┤
│                                                                  │
│  Guest memory (MOUNTFS region) ──► FAT32 extraction ──► Host dir │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

The flow has three phases:

1. **Launch.** The host builds a FAT32 image from `<host-dir>`, packs it (along with an optional
   `-ramfs` image) into a multi-image container, and maps the container into guest memory via the
   existing RamFs/MMIO infrastructure.
2. **Runtime.** The guest runtime detects the multi-image format, parses the header, and mounts each
   sub-image as a separate VFS mount point. Guest applications read and write files in-memory
   through the FAT32 backend.
3. **Shutdown.** The host reads the MOUNTFS region back from guest memory,
   opens it as a FAT filesystem, and recursively extracts all files to
   `<host-dir>`, overwriting originals.

### 3.2 Multi-Image Binary Format

To support multiple filesystem images in a single MMIO region, a lightweight container format is
used:

```text
┌─────────────────────────────────────────────────────────────────┐
│ HEAD Page (4096 bytes)                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ magic:       u32 = 0x4D494D47 ("MIMG")                     │ │
│  │ version:     u32 = 1                                       │ │
│  │ num_images:  u32                                           │ │
│  │ _pad0:       u32                                           │ │
│  │ total_size:  u64 (including HEAD page)                     │ │
│  │ reserved:    [u8; 16] (zero-padded, future use)            │ │
│  ├────────────────────────────────────────────────────────────┤ │
│  │ ImageEntry[0]: tag[8] | offset: u64 | size: u64 | flags    │ │
│  │ ImageEntry[1]: tag[8] | offset: u64 | size: u64 | flags    │ │
│  │ ...                                                        │ │
│  │ (padding to 4096 bytes)                                    │ │
│  └────────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│ Sub-image 0 (page-aligned)                                      │
├─────────────────────────────────────────────────────────────────┤
│ Sub-image 1 (page-aligned)                                      │
└─────────────────────────────────────────────────────────────────┘
```

#### Header Fields

| Field       | Type     | Description                              |
|-------------|----------|------------------------------------------|
| magic       | u32      | `0x4D494D47` ("MIMG" in LE)              |
| version     | u32      | Format version (currently `1`)           |
| num_images  | u32      | Number of sub-images                     |
| _pad0       | u32      | Alignment padding                        |
| total_size  | u64      | Total container size including HEAD page |
| reserved    | [u8; 16] | Reserved for future use                  |

#### Entry Fields (32 bytes each)

| Field  | Type    | Description                           |
|--------|---------|---------------------------------------|
| tag    | [u8; 8] | Identifies the sub-image              |
| offset | u64     | Byte offset from container start      |
| size   | u64     | Sub-image size in bytes               |
| flags  | u32     | Bitfield (bit 0 = FLAG_READONLY)      |
| _pad   | u32     | Alignment padding                     |

#### Tags

- `TAG_ROOTFS  = b"ROOTFS  "` — Root filesystem (mounted at `/`)
- `TAG_MOUNTFS = b"MOUNTFS "` — Host-mounted directory (mounted at `/mnt`)

#### Limits

- Maximum entries: `(4096 - 40) / 32 = 126`
- Sub-images are page-aligned (4096 bytes)

### 3.3 Size Constraints

| Constraint | Value | Source |
| --- | --- | --- |
| `MEMORY_SIZE` | 128 MiB | `kernel_config.toml` |
| Max mount image | 16 MiB | `MEMORY_SIZE / 8` |
| `MIN_IMAGE_SIZE` | 1 MiB | `mkramfs` constant |
| `HEADROOM_FACTOR` | 1.5x | `mkramfs` constant |
| `HEAD_PAGE_SIZE` | 4096 bytes | `multiimage` constant |
| `MAX_ENTRIES` | 126 | `(4096 - 40) / 32` |
| `RAMFS_MIN_SLACK_BYTES` | 4 MiB | Between initrd end and RAMFS start |

### 3.4 Edge Cases

1. **Empty host directory** — Creates a minimal 1 MiB formatted (empty) FAT image.
2. **No `-ramfs` with `-mount`** — Container holds only MOUNTFS (no ROOTFS).
3. **No `-mount` flag** — Existing behavior preserved (single-image path).
4. **Host directory too large** — Error with clear message when content exceeds 16 MiB.
5. **Non-existent directory** — Rejected at CLI parse time with descriptive error.
6. **Legacy guest binary** — Magic-byte check returns false; guest mounts the entire region at `/`
   as before (backward compatible).

### 3.5 Backward Compatibility

When only `-ramfs` is provided (no `-mount`), the existing single-image code path is preserved
unchanged. Guest detection uses magic-byte checking: old single images don't match the MIMG magic,
so the legacy mount-at-root path is taken. No changes to kernel MMIO infrastructure, control
register layout, or the `"RAMFS   "` tag mechanism.

### 3.6 Limitations

This approach is intentionally simple but has fundamental constraints that limit the workloads it
can support:

| Limitation | Impact |
| --- | --- |
| Boot/shutdown copy overhead | O(n) latency proportional to host directory size |
| 16 MiB size ceiling | Cannot mount directories larger than guest memory budget |
| No live host→guest sync | Host changes after launch are invisible to guest |
| No live guest→host sync | Guest changes invisible to host until shutdown |
| Full memory copy of host dir | Wastes guest memory on data that may never be accessed |
