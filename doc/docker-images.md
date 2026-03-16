# Building Nanvix OCI Images

This guide explains how to build OCI-compliant container images for Nanvix workloads.
These images are used with the `containerd-shim-nanvix-v1` runtime.

For full details on the image format, annotations, and layer structure, see
[Nanvix OCI Image Specification](nanvix-oci-image-spec.md).

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
# ...
```

### Cross-compiling your own application

Use the Nanvix cross-compiler toolchain to build your application:

```bash
# Rust (no_std)
cargo +nanvix-x86 build --target i686-unknown-nanvix --release
cp target/i686-unknown-nanvix/release/myapp myapp.elf

# C
i686-nanvix-gcc -o myapp.elf main.c
```

See the [Nanvix documentation](https://github.com/nanvix/nanvix) for details on the toolchain
and supported targets.

### Preparing the build context

Once you have your `.elf` binary, place it alongside a `Dockerfile` in a build directory:

```
build-ctx/
├── Dockerfile
├── myapp.elf
└── sysroot/       # (optional) filesystem contents for ramfs
    ├── lib/
    └── data/
```

## Minimal Image (No Ramfs)

For self-contained applications that don't need a filesystem:

```dockerfile
FROM scratch
COPY myapp.elf /initrd/myapp.elf
LABEL com.nanvix.os="nanvix"
LABEL com.nanvix.arch="x86"
LABEL com.nanvix.initrd.path="/initrd/myapp.elf"
ENTRYPOINT ["/initrd/myapp.elf"]
```

Build and run:

```bash
# Place your .elf and the Dockerfile above in a directory, then:
docker build -t myapp:latest .
docker save myapp:latest | sudo ctr images import -
sudo ctr run --rm --runtime io.containerd.nanvix.v1 \
  --label com.nanvix.os=nanvix \
  --label com.nanvix.arch=x86 \
  --label com.nanvix.initrd.path=/initrd/myapp.elf \
  docker.io/library/myapp:latest myapp-test
```

## Image With Ramfs

For applications that need shared libraries, data files, or configuration. The shim invokes
`mkramfs` at runtime to convert the `/ramfs/` tree into a FAT32 image for the VM.

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

Build and run:

```bash
docker build -t myapp-ramfs:latest .
docker save myapp-ramfs:latest | sudo ctr images import -
sudo ctr run --rm --runtime io.containerd.nanvix.v1 \
  --label com.nanvix.os=nanvix \
  --label com.nanvix.arch=x86 \
  --label com.nanvix.initrd.path=/initrd/myapp.elf \
  --label com.nanvix.ramfs.root=/ramfs \
  docker.io/library/myapp-ramfs:latest myapp-ramfs-test
```

## Annotation Reference

| Annotation | Required | Description |
|------------|----------|-------------|
| `com.nanvix.os` | Yes | Target OS (always `"nanvix"`) |
| `com.nanvix.arch` | Yes | Target architecture (`"x86"`) |
| `com.nanvix.initrd.path` | Yes | Path to the application binary within the image |
| `com.nanvix.ramfs.root` | No | Path to the ramfs directory. If absent, no ramfs is attached. |
| `com.nanvix.initrd.args` | No | Arguments passed to the application (space-separated) |
| `com.nanvix.initrd.env` | No | Environment variables (`"KEY1=val1 KEY2=val2"`) |
| `com.nanvix.execution-mode` | No | Execution mode override (`"standalone"`, `"hyperlight"`). Uses host default if absent. |
| `com.nanvix.version` | No | Nanvix version compatibility hint |

> **Note:** When using `ctr run`, annotations from Docker `LABEL` directives are not
> automatically propagated to the OCI runtime spec. You must pass them explicitly via
> `--label` flags as shown above.