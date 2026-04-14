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

# Deployment mode: standalone, single-process, multi-process, l2
export DEPLOYMENT_MODE ?= multi-process

# Validate DEPLOYMENT_MODE.
VALID_DEPLOYMENT_MODES := standalone single-process multi-process l2
ifeq ($(filter $(DEPLOYMENT_MODE),$(VALID_DEPLOYMENT_MODES)),)
$(error Invalid DEPLOYMENT_MODE '$(DEPLOYMENT_MODE)'. Valid values: $(VALID_DEPLOYMENT_MODES))
endif

# Log Level
ifeq ($(RELEASE),yes)
export LOG_LEVEL ?= error
else
export LOG_LEVEL ?= trace
endif

# Wasm binary to embed in the WASM Daemon
export WASM_BINARY ?= $(BINARIES_DIR)/hello-wasm.wasm
export WASM_BINARY_ARGS ?= ""

# Wasm Daemon Socket Address
export WASMD_SOCKADDR ?= 127.0.0.1:8585

# Default System Image
export IMAGE ?= nanvix.img

# Enable WHP backend?
export WHP ?= no

#===================================================================================================
# OS Detection
#===================================================================================================

# Detect the host operating system for cross-platform support.
ifeq ($(OS),Windows_NT)
  IS_WINDOWS := yes
  # Extension for host binaries in bin/ (.exe on Windows, .elf on Linux).
  export HOST_BIN_EXT := exe
  # Suffix that Cargo adds to host executables (empty on Linux, .exe on Windows).
  export CARGO_EXE_SUFFIX := .exe
else
  IS_WINDOWS :=
  export HOST_BIN_EXT := elf
  export CARGO_EXE_SUFFIX :=
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
export SNAPSHOT_DIR  := $(ROOT_DIR)/images
export LOGS_DIR      := $(ROOT_DIR)/logs
export SCRIPTS_DIR   := $(ROOT_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export CLH_DIR       ?= $(ROOT_DIR)/toolchain
export SYSROOT_DIR   ?= $(ROOT_DIR)/sysroot$(if $(filter yes,$(RELEASE)),-release,-debug)
export SYSROOT_LINK  := $(ROOT_DIR)/sysroot
export TARGETS_DIR   := $(BUILD_DIR)/targets
export OBJECTS_DIR   := $(ROOT_DIR)/target

# Targets that do not produce reusable compilation artifacts.
# Disable sccache unconditionally for these targets to avoid intermittent
# sccache server crashes inside Docker BuildKit containers (see #1395).
# The `override` directive ensures this takes effect even when SCCACHE is
# passed on the command line (e.g., from Dockerfile.build).
NO_SCCACHE_GOALS := check format format-check lint lint-check spellcheck spellcheck-fix verify help clean distclean

# On Windows (MSYS/Git-for-Windows sh), `which` returns POSIX paths (e.g.
# /c/Users/...) that native Windows tools cannot resolve.  Pipe through
# `cygpath -w` when available to convert them to Windows paths.
WHICH_SCCACHE = $(shell p=$$(which sccache 2>/dev/null) && \
	if command -v cygpath >/dev/null 2>&1 && [ -n "$$p" ]; then \
		cygpath -w "$$p"; \
	else echo "$$p"; fi)

ifeq ($(MAKECMDGOALS),)
# Default target ('all') produces artifacts — enable sccache.
export SCCACHE ?= $(WHICH_SCCACHE)
else ifeq ($(filter-out $(NO_SCCACHE_GOALS),$(MAKECMDGOALS)),)
# All command-line goals are check-only — disable sccache.
override SCCACHE :=
export SCCACHE
else
# At least one goal produces artifacts — enable sccache.
export SCCACHE ?= $(WHICH_SCCACHE)
endif

#===================================================================================================
# Release Artifact Configuration
#===================================================================================================

RELEASE_DEPLOYMENT_MODE := $(subst -,_,$(DEPLOYMENT_MODE))
RELEASE_BUILD_MODE := $(if $(filter yes,$(RELEASE)),release,debug)
RELEASE_VERSION := $(strip $(shell cargo metadata --no-deps --format-version 1 2>/dev/null | sed -n 's/.*"version":"\([^"]*\)".*/\1/p' | head -n1))

# Extract memory_size (bytes) from kernel config and convert to megabytes.
MEMORY_SIZE_BYTES = $(strip $(shell sed -nE 's/^[[:space:]]*memory_size[[:space:]]*=[[:space:]]*(0x[0-9a-fA-F]+|[0-9]+).*/\1/p' $(BUILD_DIR)/kernel_config.toml | head -n1))
MEMORY_SIZE_MB = $(shell echo $$(($(MEMORY_SIZE_BYTES) / 1048576)))

# VFS benchmark image filename.
export VFS_BENCH_IMG ?= vfs-bench.img

RELEASE_ARCHIVE := nanvix-$(RELEASE_VERSION)-$(MACHINE)-$(RELEASE_DEPLOYMENT_MODE)-$(RELEASE_BUILD_MODE)-$(LOG_LEVEL)-$(MEMORY_SIZE_MB)mb.tar.bz2
MANIFEST_FILE := $(SYSROOT_DIR)/manifest.json

#===================================================================================================
# Artifacts
#===================================================================================================

# File format for guest executables (always ELF regardless of host OS).
export EXEC_FORMAT := elf
# Libraries
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a

# Binaries.
KERNEL := $(BINARIES_DIR)/kernel.$(EXEC_FORMAT)
LINUXD := $(BINARIES_DIR)/linuxd.$(HOST_BIN_EXT)
MKIMAGE := $(BINARIES_DIR)/mkimage.$(HOST_BIN_EXT)
MKRAMFS := $(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT)
NANVIXD := $(BINARIES_DIR)/nanvixd.$(HOST_BIN_EXT)
USERVM := $(BINARIES_DIR)/uservm.$(HOST_BIN_EXT)

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
# Rust Toolchain Configuration
#===================================================================================================

# Tools
CARGO_HOME ?= $(HOME)/.cargo
CARGO_HOME := $(subst \,/,$(CARGO_HOME))
export CARGO := $(CARGO_HOME)/bin/cargo
export RUSTC := $(CARGO_HOME)/bin/rustc

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
export GUEST_CARGO_BUILD_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) build $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) $(CARGO_PROFILE)
export GUEST_CARGO_CLEAN_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clean $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CHECK_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) check $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) --message-format=json
export GUEST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clippy $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_FMT_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) fmt

# Note: place cargo-native options (--no-default-features, --message-format, etc.) before
# unstable flags (-Z ...) and --target, so that cargo-verus can reuse the same argument order.
# Regular cargo accepts any order, so this is safe for all commands.
#
# Kernel cargo commands explicitly unset RUSTC_WRAPPER to disable sccache.  The kernel uses a custom
# build-std configuration and a non-standard target triple that can produce incorrect or stale
# artifacts when cached by sccache, including .S assembly files compiled during build-std.
export KERNEL_CARGO_BUILD_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) build --no-default-features $(CARGO_PROFILE) $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CLEAN_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) clean $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CHECK_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) check --no-default-features --message-format=json $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CLIPPY_CMD := RUSTC_WRAPPER= RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) clippy --no-default-features $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_FMT_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) fmt

# Cargo commands for wasm target.
export WASM_CARGO_BUILD_CMD := $(CARGO) build $(WASM_CARGO_PROFILE) --target wasm32-wasip1 --no-default-features
export WASM_CARGO_CLEAN_CMD := $(CARGO) clean --target wasm32-wasip1
export WASM_CARGO_CHECK_CMD := $(CARGO) check --target wasm32-wasip1 --message-format=json --no-default-features
export WASM_CARGO_CLIPPY_CMD := $(CARGO) clippy --target wasm32-wasip1 --no-default-features
export WASM_CARGO_FMT_CMD := $(CARGO) fmt

# Cargo commands for host target.
export HOST_CARGO_BUILD_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) build $(CARGO_PROFILE) --no-default-features
export HOST_CARGO_CLEAN_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) clean
export HOST_CARGO_CHECK_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) check --message-format=json --no-default-features
export HOST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) clippy --no-default-features
export HOST_CARGO_TEST_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) test --no-default-features
export HOST_CARGO_FMT_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) fmt

# Utility Commands
export RM_CMD := rm -f
export FORCE_RM_CMD := rm -rf
export MKDIR_CMD := mkdir -p
ifeq ($(IS_WINDOWS),yes)
export CP_CMD := cp -f
export SUDO_CMD :=
export SETCAP_CMD :=
else
export CP_CMD := cp -f --preserve
export SUDO_CMD := sudo
export SETCAP_CMD := setcap
endif

#===================================================================================================
# Verus Formal Verification
#===================================================================================================

# Path to the directory containing the Verus executable (no default; skip verification when unset).
export VERUS_EXECUTABLE_DIR ?=

# List of crates to verify with Verus.
VERUS_CRATES := bitmap slab

# Platform-specific Verus binary name.
ifeq ($(IS_WINDOWS),yes)
  VERUS_BINARY := verus.exe
else
  VERUS_BINARY := verus
endif

# Verus verification command.
# Uses RUSTC_BOOTSTRAP=1 because the Verus rustc wrapper identifies as a stable compiler
# but needs to accept -Z flags passed by -Z build-std.
ifeq ($(IS_WINDOWS),yes)
# On Windows, convert VERUS_EXECUTABLE_DIR to a Unix-style path for the MSYS2 shell
# so that drive-letter colons (e.g., C:\) are not misinterpreted as PATH separators.
  VERUS_PATH_PREFIX = $$(cygpath -u '$(VERUS_EXECUTABLE_DIR)')
else
  VERUS_PATH_PREFIX = $(VERUS_EXECUTABLE_DIR)
endif

export VERUS_VERIFY_CMD = RUSTC_BOOTSTRAP=1 RUSTFLAGS=$(KERNEL_RUST_FLAGS) \
	PATH="$(VERUS_PATH_PREFIX):$$PATH" \
	$(CARGO) verus verify --no-default-features

#===================================================================================================
# Top-Level Targets
#===================================================================================================

ALL_GUEST_STATIC_LIBS := posix
ALL_GUEST_RUST_LIBS := arch bitmap bump-allocator config elf error fat32 type-safe nvx proc raw-array slab sorted-vec static_assert sysapi syscall sysalloc syslog-macros syslog sys libc_stdlib libc_string mmio-tag vfs-bench-common
ALL_GUEST_RUST_LIBS_TEST_LIST := arch bitmap bump-allocator config elf error fat32 type-safe proc raw-array slab sorted-vec static_assert libc_string syslog-macros syslog mmio-tag

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo-rust-nostd noop-rust-nostd snapshot-rust-nostd vfs-bench-nostd
ALL_GUEST_APPLICATIONS := hello-rust-nostd
ALL_GUEST_TESTS := testd file-rust thread-rust stress-rust test-kernel test-mmio-fault linux-app arch-rust vfs-test misc-rust memory-rust network-rust c-bindings-rust
# dlfcn-rust requires PIE linking for dlopen/dlsym; the x86_64 static
# relocation model produces R_X86_64_32 relocations incompatible with PIE.
ifneq ($(TARGET),x86_64)
ALL_GUEST_TESTS += dlfcn-rust
endif
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES += $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := echo-wasm-rust hello-wasm noop-wasm-rust

ALL_HOST_RUST_LIBS := control-plane-api hwloc multibin profiler nanvix nanvix-http nanvix-registry nanvix-sandbox nanvix-sandbox-cache nanvix-terminal syscomm user-vm-api
# Host rlibs excluded on Windows:
#  - nanvix-http, nanvix-sandbox-cache: depend on Unix-only APIs.
#  - syscomm: test code references cfg(unix)-gated SocketAddr::Unix variant.
#  - nanvix-registry: test code uses std::fs::symlink (Unix-only).
ifeq ($(IS_WINDOWS),yes)
WINDOWS_EXCLUDED_HOST_RLIBS := nanvix-http nanvix-sandbox-cache syscomm nanvix-registry
ALL_HOST_RUST_LIBS := $(filter-out $(WINDOWS_EXCLUDED_HOST_RLIBS),$(ALL_HOST_RUST_LIBS))
endif
ALL_HOST_UTILS := echo-client mkimage mkramfs strace
# linuxd is only needed for multi-process and L2 deployments (Linux-only).
ifeq ($(filter standalone single-process,$(DEPLOYMENT_MODE)),)
ALL_HOST_DAEMONS := linuxd
else
ALL_HOST_DAEMONS :=
endif
ALL_HOST_BINARIES := $(ALL_HOST_UTILS) $(ALL_HOST_DAEMONS)

#===================================================================================================
# Sysroot Symlink Management
#===================================================================================================

.PHONY: update-sysroot-link
update-sysroot-link:
	@if [ -d "$(SYSROOT_DIR)" ]; then \
		ln -sfn "$(SYSROOT_DIR)" "$(SYSROOT_LINK)" 2>/dev/null || \
			echo "Note: Could not create sysroot symlink (run setup to enable Developer Mode on Windows)."; \
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
all-nanvix: all-host-binaries all-nanvixd all-uservm all-nanvix-test all-test-kernel-ramfs
# The containerd shim is not needed in standalone mode.
ifneq ($(DEPLOYMENT_MODE),standalone)
all-nanvix: all-nanvix-shim
endif
endif

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
all-nanvix: all-nanvix-bench
endif

# Performs local initialization.
init: init-repo

init-repo:
	$(MKDIR_CMD) $(BINARIES_DIR)
	$(MKDIR_CMD) $(LIBRARIES_DIR)
	$(MKDIR_CMD) $(LOGS_DIR)
	@if [ -d .git ]; then git config --local core.hooksPath .githooks; fi

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
clean: clean-host-binaries clean-nanvixd clean-uservm clean-nanvix-test clean-test-kernel-ramfs
ifneq ($(IS_WINDOWS),yes)
clean: clean-nanvix-shim
endif
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
clean: clean-nanvix-bench
endif

distclean: clean
	$(FORCE_RM_CMD) Cargo.lock
ifeq ($(IS_WINDOWS),yes)
	$(FORCE_RM_CMD) "$(OBJECTS_DIR)"
else
	if mountpoint -q "$(OBJECTS_DIR)" 2>/dev/null; then \
		find "$(OBJECTS_DIR)" -mindepth 1 -delete || { echo "Error: failed to clean $(OBJECTS_DIR) with find" >&2; exit 1; }; \
	else \
		$(FORCE_RM_CMD) "$(OBJECTS_DIR)"; \
	fi
endif
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
	@cp ${MKIMAGE} ${SYSROOT_DIR}/bin/
	@cp ${MKRAMFS} ${SYSROOT_DIR}/bin/
ifeq ($(filter standalone single-process,$(DEPLOYMENT_MODE)),)
	@cp ${LINUXD} ${SYSROOT_DIR}/bin/
	@cp ${USERVM} ${SYSROOT_DIR}/bin/
endif
endif
	@cp ${LIBPOSIX} ${SYSROOT_DIR}/lib/
	@mkdir -p ${SYSROOT_DIR}/etc/scripts/common
	@cp -r ${SCRIPTS_DIR}/common/* ${SYSROOT_DIR}/etc/scripts/common/
	@cp -r ${BUILD_DIR}/user/linker/$(TARGET)/user.ld ${SYSROOT_DIR}/lib/
	@$(MAKE_QUIET) update-sysroot-link

# Generates a JSON manifest with build metadata and git info.
.PHONY: release-generate-manifest
release-generate-manifest:
	@echo "Generating manifest $(MANIFEST_FILE)..."
	@bash $(SCRIPTS_DIR)/generate-manifest.sh $(MANIFEST_FILE) \
		$(RELEASE_VERSION) $(MACHINE) $(TARGET) $(DEPLOYMENT_MODE) $(RELEASE_BUILD_MODE) $(LOG_LEVEL) \
		$(BUILD_DIR)/kernel_config.toml

release: all install release-generate-manifest
	@test -n "$(MEMORY_SIZE_MB)" || { echo "ERROR: Failed to extract memory_size from kernel_config.toml"; exit 1; }
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
	@echo "  DEPLOYMENT_MODE  Deployment mode (default: $(DEPLOYMENT_MODE))"
	@echo "  LOG_LEVEL        Logging verbosity (default: $(LOG_LEVEL))"
	@echo "  MACHINE          Target machine type (default: $(MACHINE))"
	@echo "  MAKE_NO_PRINT    Suppress directory printing in recursive make (default: $(MAKE_NO_PRINT))"
	@echo "  PROFILER         Enable MicroVM profiler (default: $(PROFILER))"
	@echo "  RELEASE          Release build mode (default: $(RELEASE)) [impacts build time]"
	@echo "  SCCACHE          Path to compilation cache binary (default: auto-detected from PATH) [impacts build time]"
	@echo "  SYSROOT_DIR      Sysroot directory (default: $(SYSROOT_DIR))"
	@echo "  TARGET           Target architecture (default: $(TARGET))"
	@echo "  TIMEOUT          Execution timeout in seconds (default: $(TIMEOUT))"
	@echo "  CLH_DIR          Cloud-hypervisor installation directory (default: $(CLH_DIR))"
	@echo "  VERUS_EXECUTABLE_DIR  Path to directory containing the verus binary (unset: skip verification)"
	@echo ""
	@echo "Parameter Values"
	@echo "  DEPLOYMENT_MODE standalone, single-process, multi-process, l2"
	@echo "  MACHINE         hyperlight, microvm"
	@echo "  TARGET          x86, x86_64"
	@echo "  RELEASE         yes, no"
	@echo "  LOG_LEVEL       trace, debug, info, warn, error, panic"
	@echo "  PROFILER        yes, no"
	@echo "  MAKE_NO_PRINT   yes, no"

# Verifies all Verus-annotated crates.
.PHONY: verify $(addprefix verify-,$(VERUS_CRATES))
verify: $(addprefix verify-,$(VERUS_CRATES))

# Ensures the correct Verus version is installed before verification.
# When VERUS_EXECUTABLE_DIR is unset, verification is skipped.
# When set, validates that the verus binary exists at the given path.
.PHONY: ensure-verus
ensure-verus:
ifeq ($(VERUS_EXECUTABLE_DIR),)
	@echo "VERUS_EXECUTABLE_DIR is not set; skipping verification."
else
	@verus_dir="$(VERUS_EXECUTABLE_DIR)"; \
	if command -v cygpath >/dev/null 2>&1; then \
		verus_dir="$$(cygpath -u "$$verus_dir")"; \
	fi; \
	verus_path="$$verus_dir/$(VERUS_BINARY)"; \
	if [ ! -f "$$verus_path" ]; then \
		echo "Error: VERUS_EXECUTABLE_DIR is set to '$(VERUS_EXECUTABLE_DIR)' but no $(VERUS_BINARY) found there."; \
		exit 1; \
	fi; \
	if [ "$(IS_WINDOWS)" != "yes" ] && [ ! -x "$$verus_path" ]; then \
		echo "Error: $(VERUS_BINARY) at '$$verus_path' is not executable."; \
		exit 1; \
	fi; \
	echo "Using Verus installation at $(VERUS_EXECUTABLE_DIR)."
endif

# Pattern rule for verifying individual crates.
# Verification is skipped when VERUS_EXECUTABLE_DIR is unset.
$(addprefix verify-,$(VERUS_CRATES)): verify-%: ensure-verus
ifeq ($(VERUS_EXECUTABLE_DIR),)
	@true
else
	$(VERUS_VERIFY_CMD) -p $* $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
endif

# Fixes code linting issues.
ifeq ($(IS_WINDOWS),yes)
lint: rust-lint
else
lint: \
	rust-lint \
	shell-lint
endif

# Checks for linting issues in the code.
ifeq ($(IS_WINDOWS),yes)
lint-check: rust-lint-check python-lint
else
lint-check: \
	rust-lint-check \
	shell-lint-check \
	python-lint
endif

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
ifneq ($(IS_WINDOWS),yes)
rust-lint-check: rust-lint-check-nanvix-shim
endif
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
ifneq ($(IS_WINDOWS),yes)
rust-lint: rust-lint-nanvix-shim
endif
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-lint: rust-lint-nanvix-bench
endif

# Source file lists - use git ls-files if in a git repo, fall back to find.
# This is needed because Docker builds exclude the .git directory.
# On Windows (no sh.exe), the POSIX shell fallback is unavailable, so use
# git ls-files directly; the Docker find fallback is Linux-only regardless.
ifeq ($(IS_WINDOWS),yes)
PYTHON_FILES := $(shell git ls-files -- "*.py")
SHELL_FILES  := $(shell git ls-files -- "*.sh")
else
PYTHON_FILES := $(shell git ls-files -- "*.py" 2>/dev/null || find . -name "*.py" -not -path "*/venv/*" -not -path "*/.venv/*" -not -path "*/__pycache__/*" -not -path "*/toolchain/*" -not -path "*/target/*" -not -path "*/.cargo/*")
SHELL_FILES := $(shell git ls-files -- "*.sh" 2>/dev/null || find . -name "*.sh" -not -path "*/toolchain/*" -not -path "*/target/*" -not -path "*/.cargo/*")
endif

# Directories and patterns to skip during spell checking.  Duplicated on the
# command line with -S because codespell 2.2.x ignores the "skip" key when
# reading a config file via --config. Keep this list in sync with the "skip"
# entry in .codespellrc to avoid drift between the Makefile and config file.
CODESPELL_SKIP := .git,.venv,bin,lib,logs,build,target,toolchain,sysroot-debug,sysroot-release,*.pdf,*.png,*.jpg,*.jpeg,*.ico,*.svg,*.woff,*.woff2,*.eot,*.ttf

# Fixes spelling errors in source code and documentation.
# Pass --config explicitly so that .codespellrc is always honoured,
# even on codespell versions that do not auto-discover it (e.g. 2.2.x).
# Avoid passing $(ALL_SOURCE_FILES) to prevent exceeding the Windows
# command-line length limit.
spellcheck-fix:
	codespell --config .codespellrc -S "$(CODESPELL_SKIP)" --write-changes .

# Checks for spelling errors in source code and documentation.
# Pass --config explicitly so that .codespellrc is always honoured,
# even on codespell versions that do not auto-discover it (e.g. 2.2.x).
# Avoid passing $(ALL_SOURCE_FILES) to prevent exceeding the Windows
# command-line length limit.
spellcheck:
	codespell --config .codespellrc -S "$(CODESPELL_SKIP)" .

# Fixes code formatting issues.
format: \
	rust-format \
	python-format

# Checks for code formatting issues.
format-check: \
	rust-format-check \
	python-format-check

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
ifneq ($(IS_WINDOWS),yes)
rust-format: format-nanvix-shim
endif
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
ifneq ($(IS_WINDOWS),yes)
rust-format-check: format-check-nanvix-shim
endif
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
rust-format-check: format-check-nanvix-bench
endif

# Python lint variables
PYTHON_VENV_DIRECTORY=$(ROOT_DIR)/.venv

# Detect venv layout: Scripts/ on Windows, bin/ on Linux.
ifeq ($(IS_WINDOWS),yes)
PYTHON_VENV_BIN := $(PYTHON_VENV_DIRECTORY)/Scripts
PYTHON_VENV_PIP := $(PYTHON_VENV_BIN)/pip
PYTHON_VENV_PYTHON := $(PYTHON_VENV_BIN)/python
else
PYTHON_VENV_BIN := $(PYTHON_VENV_DIRECTORY)/bin
PYTHON_VENV_PIP := $(PYTHON_VENV_BIN)/pip3
PYTHON_VENV_PYTHON := $(PYTHON_VENV_BIN)/python3
endif

PY_VERBOSE :=
ifneq ($(VERBOSE),yes)
ifneq ($(IS_WINDOWS),yes)
PY_VERBOSE += >> /dev/null 2>&1
endif
endif

PYTHON_STAMP=$(PYTHON_VENV_DIRECTORY)/.requirements.stamp

python-init: $(PYTHON_STAMP)

ifeq ($(IS_WINDOWS),yes)
# On Windows, Make uses bash from Git-for-Windows; use POSIX syntax.
# Use 'python' (not 'python3') since the Windows Python launcher differs.
$(PYTHON_STAMP): $(ROOT_DIR)/requirements.txt
	@if [ ! -f "$(PYTHON_VENV_PIP)" ] && [ ! -f "$(PYTHON_VENV_PIP).exe" ]; then \
		$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY); \
		python -m venv $(PYTHON_VENV_DIRECTORY); \
	fi
	@$(PYTHON_VENV_PIP) install -r $(ROOT_DIR)/requirements.txt $(PY_VERBOSE)
	@touch $(PYTHON_STAMP)
else
$(PYTHON_STAMP): $(ROOT_DIR)/requirements.txt
	@if [ ! -f $(PYTHON_VENV_PIP) ]; then \
		$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY); \
		python3 -m venv $(PYTHON_VENV_DIRECTORY); \
	fi
	@$(PYTHON_VENV_PIP) install -r $(ROOT_DIR)/requirements.txt $(PY_VERBOSE)
	@touch $(PYTHON_STAMP)
endif

python-format: python-init
	@$(PYTHON_VENV_PYTHON) -m black $(PYTHON_FILES) $(PY_VERBOSE)

python-format-check: python-init
	@$(PYTHON_VENV_PYTHON) -m black --check $(PYTHON_FILES) $(PY_VERBOSE)

python-lint: python-init
	@$(PYTHON_VENV_PYTHON) -m flake8 $(PYTHON_FILES) $(PY_VERBOSE)

# Runs Python unit tests for the build backend (z.py).
test-python: python-init
	@$(PYTHON_VENV_PYTHON) -m unittest discover -s tests -p "test_*.py" $(PY_VERBOSE)

# Checks for linting issues in shell scripts.
shell-lint-check:
	@shellcheck -S warning $(SHELL_FILES)

# Fixes code linting issues in shell scripts.
shell-lint:
	@scripts/shell-lint-fix.sh

check: \
	check-kernel \
	check-guest-binaries \
	check-guest-rlibs \
	check-guest-staticlibs \
	check-wasmd \
	check-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
check: check-host-binaries check-host-rlibs check-nanvixd check-uservm check-nanvix-test
ifneq ($(IS_WINDOWS),yes)
check: check-nanvix-shim
endif
endif

ifneq ($(strip $(filter $(MACHINE),microvm)),)
check: check-nanvix-bench
endif

#===================================================================================================
# Build Rules for Running and Debugging
#===================================================================================================

# Runs system in release mode.
run: image
	RUST_LOG=$(LOG_LEVEL) $(NANVIXD) -console-file /dev/stdout -toolchain-bin-dir $(CLH_DIR)/bin -log-dir $(LOGS_DIR) -- $(IMAGE)

# Runs system in debug mode.
debug: image
	RUST_LOG=$(LOG_LEVEL) $(NANVIXD) -console-file /dev/stdout -toolchain-bin-dir $(CLH_DIR)/bin -log-dir $(LOGS_DIR) -- $(IMAGE)

#===================================================================================================
# Build Rules for Test Kernel RAMFS Image
#===================================================================================================

# Seed directory used to build the test-kernel-ramfs.img.
TEST_KERNEL_RAMFS_SEED := $(BINARIES_DIR)/test-kernel-ramfs-seed
TEST_KERNEL_RAMFS_IMG  := $(BINARIES_DIR)/test-kernel-ramfs.img

# Generates a FAT32 RAMFS image for the test-kernel and test-mmio-fault regression tests.
# The image is created from a small seed directory using mkramfs.
.PHONY: all-test-kernel-ramfs clean-test-kernel-ramfs
all-test-kernel-ramfs: $(TEST_KERNEL_RAMFS_IMG)

$(TEST_KERNEL_RAMFS_IMG): $(TEST_KERNEL_RAMFS_SEED)/marker.txt all-host-binaries-mkramfs
	$(MKRAMFS) -o $(TEST_KERNEL_RAMFS_IMG) $(TEST_KERNEL_RAMFS_SEED)

$(TEST_KERNEL_RAMFS_SEED)/marker.txt:
	@$(MKDIR_CMD) $(TEST_KERNEL_RAMFS_SEED)
	@echo "test-kernel ramfs marker" > $@

clean-test-kernel-ramfs:
	$(FORCE_RM_CMD) $(TEST_KERNEL_RAMFS_SEED)
	$(RM_CMD) $(TEST_KERNEL_RAMFS_IMG)

#===================================================================================================
# Build Rules for System Image
#===================================================================================================

# Builds the system image.
image: all-nanvix
	$(MKIMAGE) -o $(IMAGE) \
		$(BINARIES_DIR)/procd.$(EXEC_FORMAT)\;procd \
		$(BINARIES_DIR)/memd.$(EXEC_FORMAT)\;memd \
		$(BINARIES_DIR)/testd.$(EXEC_FORMAT)\;testd

image-clean:
	$(RM_CMD) $(IMAGE)

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

# Python unit tests for the build backend (z.py). Linux-only (venv not available on Windows).
ifneq ($(IS_WINDOWS),yes)
run-unit-tests: test-python
endif

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
# On Windows, only test uservm (other host rlibs have Unix-only test dependencies).
ifeq ($(IS_WINDOWS),yes)
run-unit-tests: test-uservm
else
run-unit-tests: test-host-rlibs
# The containerd shim tests are not needed in standalone mode.
ifneq ($(DEPLOYMENT_MODE),standalone)
run-unit-tests: test-nanvix-shim
endif
endif
endif

# Determine the test configuration file based on deployment mode and architecture.
ifeq ($(IS_WINDOWS),yes)
ifeq ($(DEPLOYMENT_MODE),standalone)
NANVIX_TEST_CONFIG := test/test-standalone-windows.toml
else
$(warning Windows host only supports 'standalone' DEPLOYMENT_MODE for tests. Using standalone Windows test configuration.)
NANVIX_TEST_CONFIG := test/test-standalone-windows.toml
endif
else
ifeq ($(DEPLOYMENT_MODE),standalone)
ifeq ($(TARGET),x86_64)
NANVIX_TEST_CONFIG := test/test-standalone-x86_64.toml
else
NANVIX_TEST_CONFIG := test/test-standalone.toml
endif
else ifneq ($(filter single-process,$(DEPLOYMENT_MODE)),)
ifeq ($(TARGET),x86_64)
NANVIX_TEST_CONFIG := test/test-standalone-x86_64.toml
else
NANVIX_TEST_CONFIG := test/test-single_process.toml
endif
else ifeq ($(DEPLOYMENT_MODE),l2)
NANVIX_TEST_CONFIG := test/test-l2.toml
else
ifeq ($(TARGET),x86_64)
NANVIX_TEST_CONFIG := test/test-standalone-x86_64.toml
else
NANVIX_TEST_CONFIG := test/test-multi_process.toml
endif
endif
endif

NANVIX_TEST_BIN := $(BINARIES_DIR)/nanvix-test.$(HOST_BIN_EXT)

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

#===================================================================================================
# Build Rules for Nanvix Shim
#===================================================================================================

include build/make/nanvix-shim.mk
