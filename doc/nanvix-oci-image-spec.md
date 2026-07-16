# Nanvix OCI Image Specification

This document describes how Nanvix workloads are packaged as OCI-compliant container images,
including the layer structure, annotation conventions, and how the shim consumes them.

> To build images in practice, see [Building Nanvix OCI Images](docker-images.md).

## Background

Nanvix is a microkernel-based OS that runs applications inside lightweight VMs.
A Nanvix workload consists of two parts:

1. **Initrd binary** — The application ELF compiled for the Nanvix target (passed to `nanvixd`
   via `-- <app.elf>`)
2. **Ramfs** — A FAT32 filesystem image containing libraries, data files, and runtime
   dependencies (passed via `-ramfs <image.img>`)

The kernel (`kernel.elf`) is **not** part of the image — it is provided by the host runtime,
analogous to how Linux containers do not bundle the Linux kernel.

## Image Structure

A Nanvix OCI image uses **standard OCI filesystem layers** (not custom media types). This
ensures full compatibility with existing registries, build tools, and CI systems.

### Layer Layout

```
OCI Image
│
├── Layer 1 (initrd): Contains the application binary
│   └── /initrd/app.elf
│
├── Layer 2..N (ramfs): Contains the filesystem tree
│   └── /ramfs/
│       ├── lib/
│       │   ├── libc.so
│       │   └── ...
│       ├── usr/
│       │   └── lib/
│       │       └── python3.12/
│       └── data/
│           └── config.txt
│
├── Config JSON:
│   ├── os: "linux"                          # pragmatic compatibility (see below)
│   ├── architecture: "amd64"
│   ├── config.Entrypoint: ["/initrd/app.elf"]
│   └── config.Labels: { com.nanvix.* annotations }
│
└── Annotations (via LABELs):
    ├── com.nanvix.os: "nanvix"              # real target OS
    ├── com.nanvix.arch: "x86"               # real target architecture
    ├── com.nanvix.initrd.path: "/initrd/app.elf"
    ├── com.nanvix.initrd.args: ""           # optional arguments
    ├── com.nanvix.ramfs.root: "/ramfs"      # directory to convert to FAT32
    └── com.nanvix.version: "0.12.166"       # optional version hint
```

### Why Two Well-Known Directories?

The `/initrd/` and `/ramfs/` directories serve distinct roles in Nanvix's execution model:

| Directory  | Purpose                                     | Maps to nanvixd flag     |
| ---------- | ------------------------------------------- | ------------------------ |
| `/initrd/` | Application binary (the program to execute) | `-- <app.elf>`           |
| `/ramfs/`  | Filesystem contents (libs, data, config)    | `-ramfs <generated.img>` |

This separation is important because:
- The initrd binary is passed directly to the VM as the entry point
- The ramfs contents are packaged into a FAT32 image by `mkramfs` before being mounted

### Platform Field: `os: "linux"` vs `os: "nanvix"`

We use `os: "linux"` in the OCI config for **pragmatic compatibility**. Many registries, scanners,
and tools only recognize `linux` and `windows`. The real target OS is conveyed via the
`com.nanvix.os` annotation.

The containerd shim identifies Nanvix workloads by the presence of `com.nanvix.*` labels, not
by the platform field. This matches the approach used by urunc/bima in the unikernel ecosystem.

## How the Shim Consumes These Images

When the containerd shim (`containerd-shim-nanvix-v1`) receives an OCI bundle:

```
1. containerd unpacks image layers → standard rootfs directory
   rootfs/
   ├── initrd/
   │   └── app.elf
   └── ramfs/
       ├── lib/
       ├── usr/
       └── data/

2. Shim reads annotations from OCI config:
   initrd_path = config.Labels["com.nanvix.initrd.path"]  → "/initrd/app.elf"
   ramfs_root  = config.Labels["com.nanvix.ramfs.root"]   → "/ramfs"
   initrd_args = config.Labels["com.nanvix.initrd.args"]  → ""

3. Shim resolves paths against unpacked rootfs:
   initrd_binary = rootfs + initrd_path  → /run/containerd/.../rootfs/initrd/app.elf
   ramfs_dir     = rootfs + ramfs_root   → /run/containerd/.../rootfs/ramfs/

4. If ramfs_root is set, shim creates FAT32 image:
   mkramfs -o /tmp/<container-id>.img <ramfs_dir>

5. Shim launches nanvixd in HTTP mode and spawns the application via the NEW API.
```

## Layer Deduplication

Because ramfs contents are stored as standard filesystem layers, Docker's layer caching and
registry deduplication work naturally:

```
nanvix-base          → Layer A: /ramfs/lib/libc.so, /ramfs/lib/libm.so
nanvix-python:3.12   → Layer A (shared) + Layer B: /ramfs/usr/lib/python3.12/
my-python-app:v1     → Layer A (shared) + Layer B (shared) + Layer C: /ramfs/app/server.py
my-python-app:v2     → Layer A (shared) + Layer B (shared) + Layer D: /ramfs/app/server.py (updated)
```

Pulling `my-python-app:v2` on a machine that already has `v1` downloads only Layer D.

## Annotation Reference

| Annotation               | Required | Description                                                   |
| ------------------------ | -------- | ------------------------------------------------------------- |
| `com.nanvix.os`          | Yes      | Target OS (always `"nanvix"`)                                 |
| `com.nanvix.arch`        | Yes      | Target architecture (`"x86"`)                                 |
| `com.nanvix.initrd.path` | Yes      | Path to the application binary within the image               |
| `com.nanvix.ramfs.root`  | No       | Path to the ramfs directory. If absent, no ramfs is attached. |
| `com.nanvix.initrd.args` | No       | Arguments passed to the application (space-separated)         |
| `com.nanvix.initrd.env`  | No       | Environment variables (`"KEY1=val1 KEY2=val2"`)               |
| `com.nanvix.version`     | No       | Nanvix version compatibility hint                             |
