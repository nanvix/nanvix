# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

.DEFAULT_GOAL := all

#===================================================================================================
# Build Options
#===================================================================================================

# Target Architecture
export TARGET ?= x86

# Target Machine
export MACHINE ?= microvm

# Release Version?
export RELEASE ?= no

# Timeout
export TIMEOUT ?= 600

# Enable Microvm profiler?
export PROFILER ?= no

# Enable message timestamping
# WARNING: use only with the echo-breakdown benchmark
export TIMESTAMP_MSG ?= no

# Target Host CPU
export HOST_CPU ?=

# L2 VM deployment?
export L2_VM ?= no

# Single-process deployment?
export SINGLE_PROCESS ?= no

# Log Level
export LOG_LEVEL ?= warn

# Wasm binary to embed in the WASM Daemon
export WASM_BINARY ?= $(BINARIES_DIR)/hello-wasm.wasm
export WASM_BINARY_ARGS ?= ""

# Wasm Daemon Socket Address
export WASMD_SOCKADDR ?= 127.0.0.1:8585

# Default System Image
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
export IMAGE ?= $(BINARIES_DIR)/noop-rust-nostd.elf
else
export IMAGE ?= nanvix.iso
endif

#===================================================================================================
# Make Configuration
#===================================================================================================

# Suppress directory printing in recursive make calls?
export MAKE_NO_PRINT ?= yes

# Make command for recursive invocations (adds --no-print-directory if MAKE_NO_PRINT=yes)
export MAKE_QUIET := $(MAKE) $(if $(filter yes,$(MAKE_NO_PRINT)),--no-print-directory)

#===================================================================================================
# Directories
#===================================================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  := $(ROOT_DIR)/bin
export LIBRARIES_DIR := $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build
export IMAGE_DIR     := $(ROOT_DIR)/image
export SNAPSHOT_DIR  := $(ROOT_DIR)/images
export LOGS_DIR      := $(ROOT_DIR)/logs
export SCRIPTS_DIR   := $(ROOT_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain
export SYSROOT_DIR   ?= $(ROOT_DIR)/sysroot$(if $(filter yes,$(RELEASE)),-release,-debug)
export SYSROOT_LINK  := $(ROOT_DIR)/sysroot
export TARGETS_DIR   := $(BUILD_DIR)/targets
export OBJECTS_DIR   := $(ROOT_DIR)/target

# Targets that do not produce reusable compilation artifacts.
# Disable sccache unconditionally for these targets to avoid intermittent
# sccache server crashes inside Docker BuildKit containers (see #1395).
# The `override` directive ensures this takes effect even when SCCACHE is
# passed on the command line (e.g., from Dockerfile.build).
NO_SCCACHE_GOALS := check format format-check lint lint-check spellcheck spellcheck-fix help clean distclean

ifeq ($(MAKECMDGOALS),)
# Default target ('all') produces artifacts — enable sccache.
export SCCACHE ?= $(shell which sccache 2>/dev/null)
else ifeq ($(filter-out $(NO_SCCACHE_GOALS),$(MAKECMDGOALS)),)
# All command-line goals are check-only — disable sccache.
override SCCACHE :=
export SCCACHE
else
# At least one goal produces artifacts — enable sccache.
export SCCACHE ?= $(shell which sccache 2>/dev/null)
endif

#===================================================================================================
# Release Artifact Configuration
#===================================================================================================

RELEASE_DEPLOYMENT_MODE := $(if $(filter yes,$(SINGLE_PROCESS)),single_process,multi_process)
RELEASE_BUILD_MODE := $(if $(filter yes,$(RELEASE)),release,debug)
RELEASE_VERSION := $(strip $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | jq -r '.packages[0].version'))
RELEASE_ARCHIVE := nanvix-$(RELEASE_VERSION)-$(MACHINE)-$(RELEASE_DEPLOYMENT_MODE)-$(RELEASE_BUILD_MODE)-$(LOG_LEVEL).tar.bz2

#===================================================================================================
# Artifacts
#===================================================================================================

# File format for executables.
export EXEC_FORMAT := elf
# Libraries
export LIBC := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
export LIBM := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libm.a
export LIBCXX := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libstdc++.a
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a

# Binaries.
KERNEL := $(BINARIES_DIR)/kernel.$(EXEC_FORMAT)
LINUXD := $(BINARIES_DIR)/linuxd.$(EXEC_FORMAT)
NANVIXD := $(BINARIES_DIR)/nanvixd.$(EXEC_FORMAT)
USERVM := $(BINARIES_DIR)/uservm.$(EXEC_FORMAT)

# Scripts
GRUB_CFG_SCRIPT := $(BUILD_DIR)/iso/boot/grub/grub.cfg

#===================================================================================================
# Nanvix Variables
#===================================================================================================

# Socket address for the WASM Daemon
ifneq ($(WASMD_SOCKADDR),)
export NANVIX_WASMD_SOCKADDR := $(WASMD_SOCKADDR)
endif

# Name of the system.
export NANVIX_SYSNAME := nanvix

# Name of the node within the communications network.
export NANVIX_NODENAME ?= localhost

# Name of the machine on which the system is running.
export NANVIX_MACHINE := $(MACHINE)

#===================================================================================================
# C Toolchain Configuration
#===================================================================================================

# Tools
export NANVIX_CC := $(TOOLCHAIN_DIR)/bin/i686-nanvix-gcc
export NANVIX_CXX := $(TOOLCHAIN_DIR)/bin/i686-nanvix-g++

# SCCACHE integration for C/C++ compilation (optional)
# Wrap every compiler entrypoint exactly once so both host and cross builds
# benefit from the cache without re-prefixing values that already include it.
ifneq ($(SCCACHE),)

# This helper ensures every compiler entrypoint picks up sccache exactly once.
define wrap_with_sccache
$(strip $(if $(filter $(SCCACHE),$(firstword $1)),$1,$(SCCACHE) $1))
endef

export CC := $(call wrap_with_sccache,$(CC))
export CXX := $(call wrap_with_sccache,$(CXX))
export NANVIX_CC := $(call wrap_with_sccache,$(NANVIX_CC))
export NANVIX_CXX := $(call wrap_with_sccache,$(NANVIX_CXX))
undefine wrap_with_sccache
endif

# C Compiler Options
export NANVIX_CFLAGS := -std=c17
export NANVIX_CFLAGS += -m32 -march=pentiumpro -Wa,-march=pentiumpro
export NANVIX_CFLAGS += -Wall -Wextra -Werror
export NANVIX_CFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export NANVIX_CFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export NANVIX_CFLAGS += -Wvla -Wredundant-decls
export NANVIX_CFLAGS += -pedantic-errors
export NANVIX_CFLAGS += -Wstack-usage=4096
export NANVIX_CFLAGS += -D__NANVIX_SYSNAME__="\"$(NANVIX_SYSNAME)\""
export NANVIX_CFLAGS += -D__NANVIX_NODENAME__="\"$(NANVIX_NODENAME)\""
export NANVIX_CFLAGS += -D__$(subst -,_,$(NANVIX_MACHINE))__

# C++ Compiler Options
export NANVIX_CXXFLAGS := -std=c++17
export NANVIX_CXXFLAGS += -m32 -march=pentiumpro -Wa,-march=pentiumpro
export NANVIX_CXXFLAGS += -Wall -Wextra -Werror
export NANVIX_CXXFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export NANVIX_CXXFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export NANVIX_CXXFLAGS += -Wvla -Wredundant-decls
export NANVIX_CXXFLAGS += -pedantic-errors
export NANVIX_CXXFLAGS += -Wstack-usage=4096
export NANVIX_CXXFLAGS += -D__NANVIX_SYSNAME__="\"$(NANVIX_SYSNAME)\""
export NANVIX_CXXFLAGS += -D__NANVIX_NODENAME__="\"$(NANVIX_NODENAME)\""
export NANVIX_CXXFLAGS += -D__$(subst -,_,$(NANVIX_MACHINE))__

# Linker Options
export NANVIX_LDFLAGS := -z noexecstack -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

# Optimization Flags
ifeq ($(RELEASE), yes)
export NANVIX_CFLAGS += -O3
export NANVIX_CXXFLAGS += -O3
export NANVIX_CFLAGS += -D__RELEASE
export NANVIX_CXXFLAGS += -D__RELEASE
else
export NANVIX_CFLAGS += -O0
export NANVIX_CFLAGS += -g
export NANVIX_CXXFLAGS += -O0
export NANVIX_CFLAGS += -D__DEBUG
export NANVIX_CXXFLAGS += -D__DEBUG
endif

#===================================================================================================
# Rust Toolchain Configuration
#===================================================================================================

# Tools
export CARGO := $(HOME)/.cargo/bin/cargo
export RUSTC := $(HOME)/.cargo/bin/rustc

# SCCACHE integration for Rust compilation (optional)
ifneq ($(SCCACHE),)
export RUSTC_WRAPPER := $(SCCACHE)
endif

# Rust flags for guest target.
export GUEST_RUST_FLAGS := "-C relocation-model=static -C prefer-dynamic=no"
export GUEST_CARGO_FLAGS := -Zbuild-std=core,alloc
export GUEST_CARGO_TARGET := --target $(TARGETS_DIR)/$(TARGET)-user.json
export KERNEL_RUST_FLAGS := "-C relocation-model=static -C prefer-dynamic=no"
# Note: use '-Z flag' (with a space) instead of '-Zflag' so that cargo-verus can parse and forward
# the flags correctly. Regular cargo accepts both forms.
export KERNEL_CARGO_FLAGS := -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem
export KERNEL_CARGO_TARGET := --target $(TARGETS_DIR)/$(TARGET)-kernel.json

# Rust flags for host target.
export HOST_RUST_FLAGS := $(if $(HOST_CPU),-C target-cpu=$(HOST_CPU))

# Optimization Flags
ifeq ($(RELEASE),yes)
export BUILD_MODE := release
export CARGO_PROFILE := --release
export WASM_CARGO_PROFILE := --profile release-wasm
export WASM_BUILD_MODE := release-wasm
else
export BUILD_MODE := debug
export CARGO_PROFILE :=
export WASM_CARGO_PROFILE := --profile dev-wasm
export WASM_BUILD_MODE := dev-wasm
endif

#===================================================================================================
# Commands
#===================================================================================================

# Cargo commands for guest target.
export GUEST_CARGO_BUILD_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) $(CARGO_PROFILE)
export GUEST_CARGO_CLEAN_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 clean $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CHECK_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 check $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) --message-format=json
export GUEST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_FMT_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

# Note: place cargo-native options (--no-default-features, --message-format, etc.) before
# unstable flags (-Z ...) and --target, so that cargo-verus can reuse the same argument order.
# Regular cargo accepts any order, so this is safe for all commands.
#
# Kernel cargo commands explicitly unset RUSTC_WRAPPER to disable sccache.  The kernel uses a custom
# build-std configuration and a non-standard target triple that can produce incorrect or stale
# artifacts when cached by sccache, including .S assembly files compiled during build-std.
export KERNEL_CARGO_BUILD_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 build --no-default-features $(CARGO_PROFILE) $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CLEAN_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clean $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CHECK_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 check --no-default-features --message-format=json $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CLIPPY_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy --no-default-features $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_FMT_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

# Cargo commands for wasm target.
export WASM_CARGO_BUILD_CMD := $(CARGO) +nanvix-x86 build $(WASM_CARGO_PROFILE) --target wasm32-wasip1 --no-default-features
export WASM_CARGO_CLEAN_CMD := $(CARGO) +nanvix-x86 clean --target wasm32-wasip1
export WASM_CARGO_CHECK_CMD := $(CARGO) +nanvix-x86 check --target wasm32-wasip1 --message-format=json --no-default-features
export WASM_CARGO_CLIPPY_CMD := $(CARGO) +nanvix-x86 clippy --target wasm32-wasip1 --no-default-features
export WASM_CARGO_FMT_CMD := $(CARGO) +nanvix-x86 fmt

# Cargo commands for host target.
export HOST_CARGO_BUILD_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(CARGO_PROFILE) --no-default-features
export HOST_CARGO_CLEAN_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 clean
export HOST_CARGO_CHECK_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 check --message-format=json --no-default-features
export HOST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy --no-default-features
export HOST_CARGO_TEST_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 test --no-default-features
export HOST_CARGO_FMT_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

# Utility Commands
export RM_CMD := rm -f
export FORCE_RM_CMD := rm -rf
export MKDIR_CMD := mkdir -p
export CP_CMD := cp -f --preserve
export GRUB_CMD := grub-mkrescue
export SUDO_CMD := sudo
export SETCAP_CMD := setcap

#===================================================================================================
# Verus Formal Verification
#===================================================================================================

# Path to the Verus installation directory.
export VERUS_DIR ?= $(TOOLCHAIN_DIR)/verus

# List of crates to verify with Verus.
VERUS_CRATES := bitmap

# Verus verification command.
# Uses RUSTC_BOOTSTRAP=1 because the Verus rustc wrapper identifies as a stable compiler
# but needs to accept -Z flags passed by -Z build-std.
export VERUS_VERIFY_CMD = RUSTC_BOOTSTRAP=1 RUSTFLAGS=$(KERNEL_RUST_FLAGS) PATH="$(VERUS_DIR):$$PATH" \
	$(CARGO) +$(RUST_CHANNEL) verus verify --no-default-features

#===================================================================================================
# Top-Level Targets
#===================================================================================================

ALL_GUEST_STATIC_LIBS := posix
ALL_GUEST_RUST_LIBS := arch bitmap config elf error fat32 type-safe nvx proc raw-array slab static_assert sysapi syscall sysalloc syslog-macros syslog sys libc_stdlib libc_string
ALL_GUEST_RUST_LIBS_TEST_LIST := arch bitmap config elf error fat32 type-safe proc raw-array slab static_assert libc_string syslog-macros syslog

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo-rust-nostd noop-rust-nostd
ALL_GUEST_APPLICATIONS := hello-rust-nostd
ALL_GUEST_TESTS := testd file-rust thread-rust stress-rust test-kernel linux-app arch-rust fat32-test
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES += $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := echo-wasm-rust hello-wasm noop-wasm-rust

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
ALL_HOST_RUST_LIBS := control-plane-api hwloc profiler nanvix nanvix-http nanvix-registry nanvix-sandbox nanvix-sandbox-cache nanvix-terminal syscomm user-vm-api
ALL_HOST_UTILS := echo-client strace
ALL_HOST_DAEMONS := linuxd
ALL_HOST_BINARIES := $(ALL_HOST_UTILS) $(ALL_HOST_DAEMONS)
else
ALL_HOST_RUST_LIBS :=
ALL_HOST_UTILS :=
ALL_HOST_DAEMONS :=
ALL_HOST_BINARIES :=
endif

#===================================================================================================
# Sysroot Symlink Management
#===================================================================================================

.PHONY: update-sysroot-link
update-sysroot-link:
	@if [ -d "$(SYSROOT_DIR)" ]; then \
		ln -sfn "$(SYSROOT_DIR)" "$(SYSROOT_LINK)"; \
		echo "Linked sysroot -> $(notdir $(SYSROOT_DIR))"; \
	else \
		echo "Warning: Sysroot directory '$(SYSROOT_DIR)' not found; skipping symlink update."; \
	fi

#===================================================================================================
# SCCACHE Statistics
#===================================================================================================

# Dumps SCCACHE statistics if SCCACHE is available.
.PHONY: dump-sccache-stats
dump-sccache-stats:
	@echo ""
	@echo "================================================================================"
	@echo "SCCACHE Statistics"
	@echo "================================================================================"
	@if [ -n "$(SCCACHE)" ] && [ -x "$(SCCACHE)" ]; then \
		$(SCCACHE) --show-stats || echo "Failed to retrieve sccache statistics."; \
	else \
		echo "SCCACHE not available or not configured."; \
	fi
	@echo "================================================================================"
	@echo ""

#===================================================================================================
# Top-Level Build Rules
#===================================================================================================

# Builds everything.
all: all-nanvix
	@$(MAKE_QUIET) update-sysroot-link
	@$(MAKE_QUIET) dump-sccache-stats

# Builds all Nanvix components.
all-nanvix: \
	init \
	all-guest-staticlibs \
	all-guest-binaries \
	all-wasmd \
	all-kernel \
	all-wasm-binaries \
	all-snapshot

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
all-nanvix: all-host-binaries all-nanvixd all-uservm all-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
all-nanvix: all-nanvix-bench
endif

# Rust toolchain channel (parsed from rust-toolchain file).
RUST_CHANNEL := $(shell grep 'channel' $(ROOT_DIR)/rust-toolchain | sed 's/.*"\(.*\)"/\1/')

# Path to the nanvix-x86 custom toolchain directory.
NANVIX_X86_TOOLCHAIN_BIN := $(HOME)/.rustup/toolchains/nanvix-x86/bin

# Path to the fallback toolchain's cargo binary (from rust-toolchain channel).
FALLBACK_CARGO := $(HOME)/.rustup/toolchains/$(RUST_CHANNEL)-x86_64-unknown-linux-gnu/bin/cargo

# Performs local initialization.
init: init-repo init-nanvix-x86-cargo

init-repo:
	$(MKDIR_CMD) $(BINARIES_DIR)
	$(MKDIR_CMD) $(LIBRARIES_DIR)
	$(MKDIR_CMD) $(LOGS_DIR)
	@if [ -d .git ]; then git config --local core.hooksPath .githooks; fi

# Workaround: ensure nanvix-x86 toolchain has a cargo binary.
# Older toolchain builds did not include cargo in stage2/bin/, causing `cargo +nanvix-x86`
# to fall back to the system default cargo which may be an incompatible version.
# This copies cargo from the nightly toolchain specified in rust-toolchain as a fallback.
init-nanvix-x86-cargo:
	@if [ ! -f $(NANVIX_X86_TOOLCHAIN_BIN)/cargo ] && [ -f $(FALLBACK_CARGO) ]; then \
		echo "[WARN] cargo not found in nanvix-x86 toolchain, copying from $(RUST_CHANNEL)..."; \
		cp -f $(FALLBACK_CARGO) $(NANVIX_X86_TOOLCHAIN_BIN)/cargo; \
		echo "[INFO] cargo copied to nanvix-x86 toolchain."; \
	fi

# Cleans build.
clean: \
	clean-guest-staticlibs \
	clean-guest-binaries \
	clean-wasmd \
	clean-kernel \
	clean-wasm-binaries \
	clean-snapshot \
	image-clean

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
clean: clean-host-binaries clean-nanvixd clean-uservm clean-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
clean: clean-nanvix-bench
endif

distclean: clean
	$(FORCE_RM_CMD) Cargo.lock
	if mountpoint -q "$(OBJECTS_DIR)" 2>/dev/null; then \
		find "$(OBJECTS_DIR)" -mindepth 1 -delete || { echo "Error: failed to clean $(OBJECTS_DIR) with find" >&2; exit 1; }; \
	else \
		$(FORCE_RM_CMD) "$(OBJECTS_DIR)"; \
	fi
	$(FORCE_RM_CMD) $(LIBRARIES_DIR)
	$(FORCE_RM_CMD) $(BINARIES_DIR)
	$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY)
	$(FORCE_RM_CMD) $(SYSROOT_DIR)
	$(FORCE_RM_CMD) $(SYSROOT_LINK)

# Installs build artifacts.
install: all-nanvix
	@echo "Installing Nanvix in ${SYSROOT_DIR}..."
	@mkdir -p ${SYSROOT_DIR}/bin
	@mkdir -p ${SYSROOT_DIR}/lib
	@mkdir -p ${SYSROOT_DIR}/etc/scripts
	@cp ${KERNEL} ${SYSROOT_DIR}/bin/
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@cp ${NANVIXD} ${SYSROOT_DIR}/bin/
ifneq ($(SINGLE_PROCESS),yes)
	@cp ${LINUXD} ${SYSROOT_DIR}/bin/
	@cp ${USERVM} ${SYSROOT_DIR}/bin/
endif
endif
	@cp ${LIBPOSIX} ${SYSROOT_DIR}/lib/
	@mkdir -p ${SYSROOT_DIR}/etc/scripts/common
	@cp -r ${SCRIPTS_DIR}/common/* ${SYSROOT_DIR}/etc/scripts/common/
	@cp -r ${BUILD_DIR}/user/linker/$(TARGET)/user.ld ${SYSROOT_DIR}/lib/
	@$(MAKE_QUIET) update-sysroot-link

release: all install
	@echo "Creating release archive ${RELEASE_ARCHIVE} from ${SYSROOT_DIR}..."
	@$(RM_CMD) ${RELEASE_ARCHIVE}
	@tar -cjf ${RELEASE_ARCHIVE} --exclude=./src -C ${SYSROOT_DIR} .

# Shows available make targets and build parameters.
help:
	@echo ""
	@echo "Main Build Targets"
	@echo "  all          Build everything (default target)"
	@echo "  clean        Remove build artifacts and intermediate files"
	@echo "  distclean    Cleans everything"
	@echo "  help         Show this help message"
	@echo "  test         Run unit and system tests sequentially"
	@echo ""
	@echo "Development Targets"
	@echo "  check           Run all validation checks (syntax, compilation)"
	@echo "  format          Fix code formatting issues automatically"
	@echo "  format-check    Check code formatting without fixing"
	@echo "  install         Install build artifacts in the sysroot directory"
	@echo "  release         Create release archive from the sysroot directory"
	@echo "  lint            Fix code linting issues automatically"
	@echo "  lint-check      Check for linting issues without fixing"
	@echo "  spellcheck      Check for spelling errors in source code and documentation"
	@echo "  spellcheck-fix  Fix spelling errors in source code and documentation"
	@echo "  verify          Run Verus formal verification on annotated crates"
	@echo ""
	@echo "Testing Targets"
	@echo "  run-unit-tests       Run unit tests for libraries and components"
	@echo "  run-nanvix-tests     Run system integration tests using nanvix-test"
	@echo ""
	@echo "Execution Targets"
	@echo "  debug    Run system in debug mode"
	@echo "  image    Build system image for deployment"
	@echo "  run      Run system in release mode"
	@echo ""
	@echo "Build Parameters (override with VAR=value, see Parameter Values section below)"
	@echo "  L2_VM            Enable L2 VM deployment (default: $(L2_VM))"
	@echo "  LOG_LEVEL        Logging verbosity (default: $(LOG_LEVEL))"
	@echo "  MACHINE          Target machine type (default: $(MACHINE))"
	@echo "  MAKE_NO_PRINT    Suppress directory printing in recursive make (default: $(MAKE_NO_PRINT))"
	@echo "  PROFILER         Enable MicroVM profiler (default: $(PROFILER))"
	@echo "  RELEASE          Release build mode (default: $(RELEASE)) [impacts build time]"
	@echo "  SCCACHE          Path to compilation cache binary (default: auto-detected from PATH) [impacts build time]"
	@echo "  SINGLE_PROCESS   Enable single-process deployment (default: $(SINGLE_PROCESS))"
	@echo "  SYSROOT_DIR      Sysroot directory (default: $(SYSROOT_DIR))"
	@echo "  TARGET           Target architecture (default: $(TARGET))"
	@echo "  TIMEOUT          Execution timeout in seconds (default: $(TIMEOUT))"
	@echo "  TOOLCHAIN_DIR    Toolchain location (default: $(TOOLCHAIN_DIR))"
	@echo "  VERUS_DIR        Path to Verus installation (default: $(VERUS_DIR))"
	@echo ""
	@echo "Parameter Values"
	@echo "  MACHINE         hyperlight, microvm, qemu-pc, qemu-isapc, qemu-baremetal"
	@echo "  TARGET          x86"
	@echo "  RELEASE         yes, no"
	@echo "  LOG_LEVEL       trace, debug, info, warn, error"
	@echo "  PROFILER        yes, no"
	@echo "  L2_VM           yes, no"
	@echo "  MAKE_NO_PRINT   yes, no"

# Verifies all Verus-annotated crates.
.PHONY: verify $(addprefix verify-,$(VERUS_CRATES))
verify: $(addprefix verify-,$(VERUS_CRATES))

# Ensures the correct Verus version is installed before verification.
.PHONY: ensure-verus
ensure-verus:
	@$(SCRIPTS_DIR)/setup/verus.sh "$(VERUS_DIR)"

# Pattern rule for verifying individual crates.
$(addprefix verify-,$(VERUS_CRATES)): verify-%: ensure-verus
	$(VERUS_VERIFY_CMD) -p $* $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)

# Fixes code linting issues.
lint: \
	rust-lint \
	shell-lint

# Checks for linting issues in the code.
lint-check: \
	rust-lint-check \
	python-lint \
	shell-lint-check

# Runs clippy.
rust-lint-check: \
	rust-lint-check-kernel \
	rust-lint-check-guest-binaries \
	rust-lint-check-guest-rlibs \
	rust-lint-check-guest-staticlibs \
	rust-lint-check-wasmd \
	rust-lint-check-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
rust-lint-check: rust-lint-check-host-binaries rust-lint-check-host-rlibs rust-lint-check-nanvixd rust-lint-check-uservm rust-lint-check-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-lint-check: rust-lint-check-nanvix-bench
endif

# Fixes code linting issues.
rust-lint: \
	rust-lint-kernel \
	rust-lint-guest-binaries \
	rust-lint-guest-rlibs \
	rust-lint-guest-staticlibs \
	rust-lint-wasmd \
	rust-lint-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
rust-lint: rust-lint-host-binaries rust-lint-host-rlibs rust-lint-nanvixd rust-lint-uservm rust-lint-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-lint: rust-lint-nanvix-bench
endif

# Source file lists - use git ls-files if in a git repo, fall back to find.
# This is needed because Docker builds exclude the .git directory.
PYTHON_FILES := $(shell git ls-files -- "*.py" 2>/dev/null || find . -name "*.py" -not -path "*/venv/*" -not -path "*/.venv/*" -not -path "*/__pycache__/*" -not -path "*/toolchain/*" -not -path "*/target/*" -not -path "*/.cargo/*")
C_CPP_FILES := $(shell git ls-files -- "*.c" "*.cpp" "*.h" "*.hpp" 2>/dev/null || find . -type f \( -name "*.c" -o -name "*.cpp" -o -name "*.h" -o -name "*.hpp" \) -not -path "*/toolchain/*" -not -path "*/target/*" -not -path "*/.cargo/*")
SHELL_FILES := $(shell git ls-files -- "*.sh" 2>/dev/null || find . -name "*.sh" -not -path "*/toolchain/*" -not -path "*/target/*" -not -path "*/.cargo/*")
ALL_SOURCE_FILES := $(shell git ls-files 2>/dev/null || find . -type f -not -path "*/target/*" -not -path "*/toolchain/*" -not -path "*/venv/*" -not -path "*/.venv/*" -not -path "*/.git/*" -not -path "*/__pycache__/*" -not -path "*/.cargo/*")

# Fixes spelling errors in source code and documentation.
spellcheck-fix:
	codespell --write-changes $(ALL_SOURCE_FILES)

# Checks for spelling errors in source code and documentation.
spellcheck:
	codespell $(ALL_SOURCE_FILES)

# Fixes code formatting issues.
format: \
	clang-format \
	python-format \
	rust-format \

# Checks for code formatting issues.
format-check: \
	clang-format-check \
	python-format-check \
	rust-format-check \

# Formats Rust code.
rust-format: \
	format-guest-binaries \
	format-guest-rlibs \
	format-guest-staticlibs \
	format-kernel \
	format-wasmd \
	format-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
rust-format: format-host-binaries format-host-rlibs format-nanvixd format-uservm format-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-format: format-nanvix-bench
endif

# Checks Rust code formatting.
rust-format-check: \
	format-check-guest-binaries \
	format-check-guest-rlibs \
	format-check-guest-staticlibs \
	format-check-kernel \
	format-check-wasmd \
	format-check-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
rust-format-check: format-check-host-binaries format-check-host-rlibs format-check-nanvixd format-check-uservm format-check-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-format-check: format-check-nanvix-bench
endif

# Python lint variables
PY_VERBOSE :=
ifneq ($(VERBOSE),yes)
PY_VERBOSE += >> /dev/null 2>&1
endif
PYTHON_VENV_DIRECTORY=$(ROOT_DIR)/.venv
PYTHON_STAMP=$(PYTHON_VENV_DIRECTORY)/.requirements.stamp

python-init: $(PYTHON_STAMP)

$(PYTHON_STAMP): $(ROOT_DIR)/requirements.txt
	@if [ ! -f $(PYTHON_VENV_DIRECTORY)/bin/pip3 ]; then \
		$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY); \
		python3 -m venv $(PYTHON_VENV_DIRECTORY); \
	fi
	@$(PYTHON_VENV_DIRECTORY)/bin/pip3 install -r $(ROOT_DIR)/requirements.txt $(PY_VERBOSE)
	@touch $(PYTHON_STAMP)

python-format: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m black $(PYTHON_FILES) $(PY_VERBOSE)

python-format-check: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m black --check $(PYTHON_FILES) $(PY_VERBOSE)

python-lint: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m flake8 $(PYTHON_FILES) $(PY_VERBOSE)

# Checks for linting issues in shell scripts.
shell-lint-check:
	@shellcheck -S warning $(SHELL_FILES)

# Fixes code linting issues in shell scripts.
shell-lint:
	@scripts/shell-lint-fix.sh

# Check C/C++ formatting style.
clang-format-check:
	@clang-format --dry-run --Werror $(C_CPP_FILES)

# Format C/C++ files.
clang-format:
	@clang-format -i $(C_CPP_FILES)

check: \
	check-kernel \
	check-guest-binaries \
	check-guest-rlibs \
	check-guest-staticlibs \
	check-wasmd \
	check-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
check: check-host-binaries check-host-rlibs check-nanvixd check-uservm check-nanvix-test
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
check: check-nanvix-bench
endif

#===================================================================================================
# Build Rules for Running and Debugging
#===================================================================================================

# Runs system in release mode.
run: image
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/run-qemu.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)
endif

# Runs system in debug mode.
debug: image
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/run-qemu.sh $(TARGET) $(MACHINE) $(IMAGE) --debug $(TIMEOUT)
endif

#===================================================================================================
# Build Rules for System Image
#===================================================================================================

# Builds the system image.
image: all-nanvix
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	$(MKDIR_CMD) $(IMAGE_DIR)/boot/grub
	$(CP_CMD) $(GRUB_CFG_SCRIPT) $(IMAGE_DIR)/boot/grub/
	$(CP_CMD) $(BINARIES_DIR)/*.$(EXEC_FORMAT) $(IMAGE_DIR)/
	$(GRUB_CMD) $(IMAGE_DIR) -o $(IMAGE)
endif

image-clean:
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	$(RM_CMD) $(IMAGE_DIR)/*.$(EXEC_FORMAT)
	$(RM_CMD) $(IMAGE)
endif

#===================================================================================================
# Build Rules for Running Tests
#===================================================================================================

.PHONY: test
test:
	@$(MAKE) run-unit-tests
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@$(MAKE) run-nanvix-tests
endif

run-unit-tests: all-nanvix test-guest-rlibs

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
run-unit-tests: test-host-rlibs
endif

# Determine the test configuration file based on deployment mode.
ifeq ($(SINGLE_PROCESS),yes)
NANVIX_TEST_CONFIG := test/test-single_process.toml
else ifeq ($(L2_VM),yes)
NANVIX_TEST_CONFIG := test/test-l2.toml
else
NANVIX_TEST_CONFIG := test/test-multi_process.toml
endif

NANVIX_TEST_BIN := $(BINARIES_DIR)/nanvix-test.elf

.PHONY: run-nanvix-tests
run-nanvix-tests: all-nanvix
	@echo "Running integration tests with configuration: $(NANVIX_TEST_CONFIG)"
	RUST_LOG=$(LOG_LEVEL) $(NANVIX_TEST_BIN) $(NANVIX_TEST_CONFIG)

#===================================================================================================
# Build Rules for L2 System VM Snapshot
#===================================================================================================

include build/make/snapshot.mk

#===================================================================================================
# Build Rules for Generic Guest Static Libraries
#===================================================================================================

include build/make/generic-guest-staticlibs.mk

#===================================================================================================
# Build Rules for Guest Rust Libraries
#===================================================================================================

include build/make/generic-guest-rlibs.mk

#===================================================================================================
# Build Rules for Generic Guest Binaries
#===================================================================================================

include build/make/generic-guest-binaries.mk

#===================================================================================================
# Build Rules for WASM Daemon Binary
#===================================================================================================

include build/make/wasmd.mk

#===================================================================================================
# Build Rules for Kernel Binary
#===================================================================================================

include build/make/kernel.mk

#===================================================================================================
# Build Rules for Generic WASM Binaries
#===================================================================================================

include build/make/generic-wasm-binaries.mk

#===================================================================================================
# Build Rules for Generic Host Rust Libraries
#===================================================================================================

include build/make/generic-host-rlibs.mk

#===================================================================================================
# Build Rules for Nanvix Bench
#===================================================================================================

include build/make/nanvix-bench.mk

#===================================================================================================
# Build Rules for Nanvix Test
#===================================================================================================

include build/make/nanvix-test.mk

#===================================================================================================
# Build Rules for Nanvix Daemon
#===================================================================================================

include build/make/nanvixd.mk

#===================================================================================================
# Build Rules for Generic Host Binaries
#===================================================================================================

include build/make/generic-host-binaries.mk

#===================================================================================================
# Build Rules for UserVM
#===================================================================================================

include build/make/uservm.mk
