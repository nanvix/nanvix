# Building Nanvix OCI Images

This guide shows how to build and run OCI container images for Nanvix workloads.

> For details on the image format, layer structure, and annotation semantics, see the
> [Nanvix OCI Image Specification](nanvix-oci-image-spec.md).

## Prerequisites

- Docker installed (see [setup.md](setup.md))
- Nanvix built — application binaries (`.elf`) available (see [setup.md](setup.md))
- The shim installed and configured (see [run.md](run.md))

## Obtaining Application Binaries

Nanvix OCI images package **pre-compiled ELF binaries** that target the Nanvix kernel. These
binaries are not standard Linux executables — they are cross-compiled for the Nanvix `i686-nanvix`
target.

There are several ways to obtain them:

### From a Nanvix build

After building Nanvix (see [setup.md](setup.md)), example binaries are available in the `bin/`
directory of the Nanvix source tree:

```bash
ls $NANVIX_DIR/bin/*.elf
# hello-rust-nostd.elf  — "Hello, world!" (no std, no filesystem)
# vfs-test.elf          — VFS test (requires ramfs)
# echo-c.elf            — Echo server (C)
```

### Cross-compiling your own application

Use the Nanvix cross-compiler toolchain:

```bash
# Rust (no_std)
cargo +nanvix-x86 build --target i686-unknown-nanvix --release
cp target/i686-unknown-nanvix/release/myapp myapp.elf

# C
i686-nanvix-gcc -o myapp.elf main.c
```

See the [Nanvix documentation](https://github.com/nanvix/nanvix) for toolchain details.

### Preparing the build context

Place your `.elf` binary alongside a `Dockerfile` in a build directory:

```
build-ctx/
├── Dockerfile
├── myapp.elf
└── sysroot/       # (optional) filesystem contents for ramfs
    ├── lib/
    └── data/
```

## Dockerfile Examples

### Minimal Image (No Ramfs)

For self-contained applications that don't need a filesystem:

```dockerfile
FROM scratch
COPY myapp.elf /initrd/myapp.elf
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
ENTRYPOINT ["/initrd/myapp.elf"]
```

### Image With Ramfs

For applications that need shared libraries, data files, or configuration:

```dockerfile
FROM scratch
COPY myapp.elf /initrd/myapp.elf
COPY sysroot/ /ramfs/
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
LABEL com.nanvix.ramfs.root="/ramfs"
ENTRYPOINT ["/initrd/myapp.elf"]
```

### Multi-Stage Build (Compile + Package)

Cross-compile inside a build container, then package for Nanvix:

```dockerfile
FROM nanvix-sdk:latest AS builder
COPY src/ /build/src/
RUN nanvix-cc -o /build/app.elf /build/src/main.c

FROM scratch
COPY --from=builder /build/app.elf /initrd/app.elf
COPY config/ /ramfs/etc/
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/app.elf"
LABEL com.nanvix.ramfs.root="/ramfs"
ENTRYPOINT ["/initrd/app.elf"]
```

### Base Image With Shared Libraries

```dockerfile
# nanvix-python:3.12 base image
FROM scratch
COPY python-sysroot/lib/ /ramfs/lib/
COPY python-sysroot/usr/ /ramfs/usr/
COPY python-runner.elf /initrd/python-runner.elf
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/python-runner.elf"
LABEL com.nanvix.ramfs.root="/ramfs"
ENTRYPOINT ["/initrd/python-runner.elf"]
```

Application images inherit the base layers:

```dockerfile
FROM nanvix-python:3.12
COPY server.py /ramfs/app/server.py
COPY myapp.elf /initrd/myapp.elf
LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
LABEL com.nanvix.initrd.args="/app/server.py"
ENTRYPOINT ["/initrd/myapp.elf"]
```

## Building and Running

```bash
# Build
docker build -t myapp:latest .

# Import into containerd
docker save myapp:latest | sudo ctr images import -

# Run with the Nanvix shim
sudo ctr run --rm --runtime io.containerd.nanvix.v1 \
  --label com.nanvix.os=nanvix \
  --label com.nanvix.arch=x86 \
  --label com.nanvix.initrd.path=/initrd/myapp.elf \
  docker.io/library/myapp:latest myapp-test
```

For images with ramfs, add the ramfs label:

```bash
sudo ctr run --rm --runtime io.containerd.nanvix.v1 \
  --label com.nanvix.os=nanvix \
  --label com.nanvix.arch=x86 \
  --label com.nanvix.initrd.path=/initrd/myapp.elf \
  --label com.nanvix.ramfs.root=/ramfs \
  docker.io/library/myapp-ramfs:latest myapp-ramfs-test
```

> **Note:** `ctr run` does not propagate Docker `LABEL` directives into the OCI runtime spec
> automatically. You must pass `com.nanvix.*` annotations via `--label` flags.

## Further Reading

- [Nanvix OCI Image Specification](nanvix-oci-image-spec.md) — Layer structure, annotation
  semantics, platform compatibility, and how the shim consumes images.