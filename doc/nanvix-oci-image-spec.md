# Nanvix OCI Image Specification

This document describes how Nanvix workloads are packaged as OCI-compliant container images,
including the layer structure, annotation conventions, and Dockerfile patterns.

## Background

Nanvix is a microkernel-based OS that runs applications inside lightweight VMs (Hyperlight).
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
│       │   ├── libssl.so
│       │   └── ...
│       ├── usr/
│       │   └── lib/
│       │       └── python3.12/
│       ├── data/
│       │   └── config.txt
│       └── ...
│
├── Config JSON:
│   ├── os: "linux"                          # pragmatic compatibility (see note below)
│   ├── architecture: "amd64"
│   ├── config.Entrypoint: ["/initrd/app.elf"]
│   └── config.Labels: { com.nanvix.* annotations }
│
└── Annotations (via LABELs):
    ├── com.nanvix.os: "nanvix"              # real target OS
    ├── com.nanvix.arch: "x86"               # real target architecture
    ├── com.nanvix.initrd.path: "/initrd/app.elf"
    ├── com.nanvix.initrd.args: ""           # optional arguments for the app
    ├── com.nanvix.ramfs.root: "/ramfs"      # directory to convert to FAT32
    └── com.nanvix.version: "0.12.166"       # optional Nanvix version hint
```

### Why Two Well-Known Directories?

The `/initrd/` and `/ramfs/` directories serve distinct roles in Nanvix's execution model:

| Directory | Purpose | Maps to nanvixd flag |
|-----------|---------|---------------------|
| `/initrd/` | Application binary (the program to execute) | `-- <app.elf>` |
| `/ramfs/` | Filesystem contents (libs, data, config) | `-ramfs <generated.img>` |

This separation is important because:
- The initrd binary is passed directly to the VM as the entry point
- The ramfs contents are packaged into a FAT32 image by `mkramfs` before being mounted

### Platform Field: `os: "linux"` vs `os: "nanvix"`

We use `os: "linux"` in the OCI config for **pragmatic compatibility**. Many registries, scanners,
and tools only recognize `linux` and `windows`. The real target OS is conveyed via the
`com.nanvix.os` annotation.

The containerd shim identifies Nanvix workloads by the presence of `com.nanvix.*` labels, not
by the platform field. This matches the approach used by urunc/bima in the unikernel ecosystem.

In the future, if `nanvix` is registered as a recognized OS value in the OCI spec, images can
use `os: "nanvix"` natively. This would be a non-breaking change.

## Dockerfile Patterns

### Minimal Image (Static Binary + Data)

The simplest case: a pre-compiled Nanvix binary with some data files.

```dockerfile
FROM scratch

# Application binary
COPY myapp.elf /initrd/myapp.elf

# Filesystem contents
COPY sysroot/ /ramfs/

# Nanvix annotations
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
LABEL com.nanvix.ramfs.root="/ramfs"

ENTRYPOINT ["/initrd/myapp.elf"]
```

Build and push:
```bash
docker build -t registry.io/myapp:v1 .
docker push registry.io/myapp:v1
```

### Multi-Stage Build (Compile + Package)

Cross-compile the application inside a build container, then package for Nanvix.

```dockerfile
# ---- Stage 1: Cross-compile ----
FROM nanvix-sdk:latest AS builder
COPY src/ /build/src/
RUN nanvix-cc -o /build/app.elf /build/src/main.c

# ---- Stage 2: Package for Nanvix ----
FROM scratch

COPY --from=builder /build/app.elf /initrd/app.elf
COPY config/ /ramfs/etc/

LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/app.elf"
LABEL com.nanvix.ramfs.root="/ramfs"

ENTRYPOINT ["/initrd/app.elf"]
```

### Using a Base Image (Shared Libraries)

When a base image provides shared libraries (e.g., Python runtime), the application image
inherits those layers and adds only its own code.

```dockerfile
# Base image provides /ramfs/lib/, /ramfs/usr/, etc.
FROM nanvix-python:3.12

# Add application code to the ramfs
COPY server.py /ramfs/app/server.py
COPY requirements/ /ramfs/app/requirements/

# Override initrd with the application entry point
COPY myapp.elf /initrd/myapp.elf

LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
LABEL com.nanvix.initrd.args="/app/server.py"

ENTRYPOINT ["/initrd/myapp.elf"]
```

The `nanvix-python:3.12` base image Dockerfile would look like:

```dockerfile
FROM scratch

# Cross-compiled Python runtime and dependencies
COPY python-sysroot/lib/    /ramfs/lib/
COPY python-sysroot/usr/    /ramfs/usr/
COPY python-sysroot/etc/    /ramfs/etc/

# Default initrd (can be overridden by child images)
COPY python-runner.elf /initrd/python-runner.elf

LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/python-runner.elf"
LABEL com.nanvix.ramfs.root="/ramfs"
LABEL com.nanvix.version="0.12.166"

ENTRYPOINT ["/initrd/python-runner.elf"]
```

### Image With No Ramfs

Some Nanvix applications are self-contained and don't need a filesystem. In this case,
omit the ramfs annotations:

```dockerfile
FROM scratch

COPY hello.elf /initrd/hello.elf

LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/hello.elf"

ENTRYPOINT ["/initrd/hello.elf"]
```

The shim detects that `com.nanvix.ramfs.root` is absent and launches nanvixd without `-ramfs`.

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

5. Shim launches nanvixd:
   nanvixd.elf -ramfs /tmp/<container-id>.img -- <initrd_binary> <initrd_args>
```

## Layer Deduplication in Practice

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

| Annotation | Required | Description |
|------------|----------|-------------|
| `com.nanvix.os` | Yes | Target OS (always `"nanvix"`) |
| `com.nanvix.arch` | Yes | Target architecture (`"x86"`) |
| `com.nanvix.initrd.path` | Yes | Path to the application binary within the image |
| `com.nanvix.ramfs.root` | No | Path to the ramfs directory within the image. If absent, no ramfs is attached. |
| `com.nanvix.initrd.args` | No | Arguments passed to the application (space-separated) |
| `com.nanvix.initrd.env` | No | Environment variables (`"KEY1=val1 KEY2=val2"`) |
| `com.nanvix.execution-mode` | No | Execution mode override (e.g., `"standalone"`, `"hyperlight"`). Uses host default if absent. |
| `com.nanvix.version` | No | Nanvix version compatibility hint |
