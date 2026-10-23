# Nanvix Build System

This directory holds the build configuration that all Nanvix components share. You don't run
anything here directly: `./z build` on Linux and `.\z.ps1 build` on Windows run GNU Make in the
repository root, and the top-level [`Makefile`](../Makefile) and the Cargo build scripts read these
files. For build commands and parameters, see the [build guide](../doc/build.md).

## Layout

| Path                                       | Contents                                          |
| ------------------------------------------ | ------------------------------------------------- |
| [`kernel/linker/`](kernel/linker)          | Kernel linker script templates.                   |
| [`kernel_config.toml`](kernel_config.toml) | Kernel build-time constants.                      |
| [`make/`](make)                            | Make fragments with each component's build rules. |
| [`make/lists/`](make/lists)                | Lists of guest components to build.               |
| [`targets/`](targets)                      | Rust target specifications.                       |
| [`user/`](user)                            | Build and linker scripts for guest programs.      |
| [`verus-version`](verus-version)           | Verus release used for formal verification.       |

## Make Fragments

The top-level `Makefile` defines the build parameters, such as `TARGET`, `MACHINE`, and `RELEASE`,
and includes the fragments in [`make/`](make). Each fragment holds the rules for one component or
family of components and is named after it: for example, [`kernel.mk`](make/kernel.mk) builds the
kernel and [`uservm.mk`](make/uservm.mk) builds the UserVM. The `generic-*` fragments generate the
same rules for each package in a list, such as the [component lists](#component-lists).

Two fragments provide the guest C runtime and toolchain.
[`nanvix-libc-artifacts.mk`](make/nanvix-libc-artifacts.mk) stages the C runtime: the static
`libc.a` and `libm.a` that C programs link against, their shared counterparts `libc.so` and
`libm.so`, and the `crt0.o` startup object. [`guest-c-apps.mk`](make/guest-c-apps.mk) defines the
C toolchain that [`posix-tests.mk`](make/posix-tests.mk) uses to build the ported POSIX C test
suites.

### Component Lists

Each file in [`make/lists/`](make/lists) defines one variable that lists guest components, one per
line, so that adding a component changes a single line. The fragments in `make/` generate rules for
every entry.

| File                                                                      | Lists                                           |
| ------------------------------------------------------------------------- | ----------------------------------------------- |
| [`guest-applications.mk`](make/lists/guest-applications.mk)               | Guest applications.                             |
| [`guest-benchmarks.mk`](make/lists/guest-benchmarks.mk)                   | Benchmark programs.                             |
| [`guest-daemons.mk`](make/lists/guest-daemons.mk)                         | System daemons.                                 |
| [`guest-tests.mk`](make/lists/guest-tests.mk)                             | Integration and system test programs.           |
| [`guest-posix-tests.mk`](make/lists/guest-posix-tests.mk)                 | Ported POSIX C test suites.                     |
| [`guest-rust-libs.mk`](make/lists/guest-rust-libs.mk)                     | Guest Rust libraries.                           |
| [`guest-rust-libs-test-list.mk`](make/lists/guest-rust-libs-test-list.mk) | Guest Rust libraries that have host unit tests. |

## Rust Target Specifications

Rust has no built-in target for Nanvix, so [`targets/`](targets) defines custom ones. The
`Makefile` builds the kernel with `<TARGET>-kernel.json` and guest programs and libraries with
`<TARGET>-user.json`, where `TARGET` is `x86` (the default) or `x86_64`. No prebuilt standard
library exists for these targets, so Cargo builds `core` and `alloc` from source with
`-Z build-std`, a nightly-only feature; [`rust-toolchain`](../rust-toolchain) pins the nightly
toolchain.

[`x86_64-user-pic.json`](targets/x86_64-user-pic.json) matches `x86_64-user.json` except for its
position-independent relocation model. On x86_64, the build compiles the C runtime for `libc.so`
and `libm.so`, and the `test-rust-dlfcn` test, against this target. The separate target gives these
position-independent builds their own Cargo output directory, apart from the static build. Keep the
two files in sync.

## Linker Scripts

The kernel linker scripts in [`kernel/linker/`](kernel/linker) are templates.
[`src/kernel/build.rs`](../src/kernel/build.rs) replaces their `@...@` placeholders with
machine-specific values and passes the generated `kernel.ld` to the linker.

[`user/linker/`](user/linker) holds the linker script for guest programs, one per architecture. The
`install` target also copies it to `lib/` in the sysroot, next to the C runtime.

## Shared Guest Build Script

[`user/build.rs`](user/build.rs) is the build script of guest programs that need no custom build
logic. Rather than copying it, each such crate links to it through a `build.rs` symbolic link. Like
the other symbolic links in this repository, these need
[Developer Mode](../doc/setup-windows.md#2-enable-developer-mode) on Windows.

The script passes the guest linker script for the target architecture to the linker. If the crate
has a `config.json` file, the script also embeds it, with spaces and newlines removed, in the
`CONFIG` compile-time environment variable.

## Kernel Configuration

[`kernel_config.toml`](kernel_config.toml) sets the kernel's build-time constants, such as the
number of processors, the kernel stack size, the timer and scheduler frequencies, the IPC message
size, and the thread and process limits. The file documents each key, including any constraints on
its value. [`src/libs/config/build.rs`](../src/libs/config/build.rs) validates the values and turns
them into constants of the `config` crate, which the kernel, daemons, and system libraries use.

## Verus Version

[`verus-version`](verus-version) pins the [Verus](https://github.com/verus-lang/verus) release used
to formally verify selected crates. [`scripts/setup/verus.py`](../scripts/setup/verus.py) installs
that release, both for `./z setup --verus` (`.\z.ps1 setup --verus` on Windows) and in CI.
[`scripts/update-verus.sh`](../scripts/update-verus.sh) bumps it, together with the `vstd` pin in
[`Cargo.toml`](../Cargo.toml). To install Verus and run verification, see the
[Linux](../doc/build-linux.md#formal-verification-with-verus) or
[Windows](../doc/build-windows.md#formal-verification-with-verus) build guide.
