# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Shared Guest C Toolchain (build guest C sources against the bundled libc)
#===================================================================================================
#
# Defines the host C toolchain and link flags used to build guest C sources
# against the merged `libc.a` (the C library + the Nanvix system-call backend,
# produced by `nanvix-libc-bundle`). These definitions are consumed by
# `build/make/posix-tests.mk`, which compiles the ported POSIX C test suites with
# them. The resulting binaries exercise the bundled libc and are never shipped in
# releases: `install`/`release` copy only the kernel, daemons, libraries, and
# host tools.
#
# C toolchain: guest C sources are cross-compiled with the host `clang`
# (targeting the i686 guest ABI), so they build on developer machines and CI
# without a dedicated cross toolchain.

#---------------------------------------------------------------------------------------------------
# C compiler.
#---------------------------------------------------------------------------------------------------

# Host clang, cross-compiling to the active guest ABI: i686 for TARGET=x86 and
# x86-64 for TARGET=x86_64. Both are freestanding `-unknown-none` targets,
# mirroring the Rust guest target specs in build/targets/$(TARGET)-user.json.
ifeq ($(TARGET),x86_64)
GUEST_C_APP_CC := clang --target=x86_64-unknown-none
else
GUEST_C_APP_CC := clang --target=i686-unknown-none
endif

#---------------------------------------------------------------------------------------------------
# Flags and inputs.
#---------------------------------------------------------------------------------------------------

# Compile flags: freestanding guest C, project headers only.
#
# Use -nostdinc for maximal isolation: it drops BOTH the host system include
# paths and the compiler's builtin resource-directory headers. The freestanding
# headers normally supplied by the compiler (stdalign.h, stdarg.h, stdbool.h,
# stddef.h, stdint.h) are vendored in-tree under include/, so the build is
# hermetic and does not depend on the compiler's resource directory (which clang
# 18 omits under -nostdinc on both Linux and Windows).
#
# The flavor follows TARGET and mirrors build/targets/$(TARGET)-user.json so the
# C objects share the Rust guest ABI:
#   * x86_64: baseline x86-64 CPU, no red zone (the kernel's exception/signal
#     delivery does not preserve it) and the small code model + non-PIE static
#     link at the guest BASE_ADDR (< 2 GiB, so 32-bit absolute relocations hold).
#   * x86: i686 / pentiumpro.
ifeq ($(TARGET),x86_64)
GUEST_C_APP_CFLAGS := -m64 -march=x86-64 -mno-red-zone -mcmodel=small -ffreestanding -nostdinc -std=c17
else
GUEST_C_APP_CFLAGS := -m32 -march=pentiumpro -ffreestanding -nostdinc -std=c17
endif
GUEST_C_APP_CFLAGS += -isystem $(ROOT_DIR)/include
ifeq ($(RELEASE),yes)
GUEST_C_APP_CFLAGS += -O3
else
GUEST_C_APP_CFLAGS += -O0 -g
endif

# Link: host ld over the merged libc.a + crt0 + the guest user linker script.
# Mirrors the proven guest link (-z muldefs == the guest build's
# -Wl,--allow-multiple-definition: both libc and the backend define __errno_location).
#
# Windows has no GNU ld. The LLVM toolchain that already supplies the guest
# `clang` also ships `ld.lld`, which performs the same ELF i386 link
# (-melf_i386, -z muldefs, -T script, --entry), so default to it there.
ifeq ($(IS_WINDOWS),yes)
GUEST_C_APP_LD ?= ld.lld
else
GUEST_C_APP_LD ?= ld
endif
GUEST_C_APP_LIBC := $(NANVIX_LIBC_BUNDLE_AR)
# Standalone math archive (newlib-style libc.a / libm.a split). Produced by
# `all-guest-staticlibs` from the `nanvix_libm` crate (libc_math + nvx panic
# handler, no sysalloc).
GUEST_C_APP_LIBM := $(LIBRARIES_DIR)/libm.a
GUEST_C_APP_LD_SCRIPT := $(BUILD_DIR)/user/linker/$(TARGET)/user.ld
# ELF flavor follows TARGET (-melf_i386 for x86, -melf_x86_64 for x86_64).
ifeq ($(TARGET),x86_64)
GUEST_C_APP_LDFLAGS := -melf_x86_64 -z noexecstack -z muldefs
else
GUEST_C_APP_LDFLAGS := -melf_i386 -z noexecstack -z muldefs
endif
# --no-warn-rwx-segments silences a GNU ld >= 2.39 diagnostic. ld.lld neither
# emits that warning nor recognizes the flag, so pass it only to GNU ld.
ifneq ($(IS_WINDOWS),yes)
GUEST_C_APP_LDFLAGS += --no-warn-rwx-segments
endif
GUEST_C_APP_LDFLAGS += -T $(GUEST_C_APP_LD_SCRIPT) --entry=_do_start
