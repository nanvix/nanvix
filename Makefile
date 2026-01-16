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

# Build optional software?
export BUILD_OPT ?= yes

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
# Optional Software Repositories (URLs and pinned commits)
#===================================================================================================

OPENBLAS_REPOSITORY := https://github.com/nanvix/OpenBLAS
OPENBLAS_COMMIT := 3230819ef84ab9340c61e0b1ae0917ec436000a4

OPENSSL_REPOSITORY := https://github.com/nanvix/openssl
OPENSSL_COMMIT := a94f3958e16321bab0b9b2b68b89ee7c59734c13

PYTHON_REPOSITORY := https://github.com/nanvix/cpython
PYTHON_COMMIT := faed6d55cb7e6332021bb9e5727f1337154aa801

SQLITE_REPOSITORY := https://github.com/nanvix/sqlite
SQLITE_COMMIT := 6a29fea94d2514ac56b1bcbddf18c62361362431

ZLIB_REPOSITORY := https://github.com/nanvix/zlib
ZLIB_COMMIT := 5166ca8c8b563b55fb3e8e2e0b157e36a3bbdcf6

QUICKJS_REPOSITORY := https://github.com/nanvix/quickjs
QUICKJS_COMMIT := efaa09fa2a28f6884185c27e74b3a731936058b5

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
export SCCACHE       ?= $(shell which sccache 2>/dev/null)

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
RUN_NANVIXD_SCRIPT := $(SCRIPTS_DIR)/run-nanvixd.sh
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
export KERNEL_CARGO_FLAGS := -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
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

export KERNEL_CARGO_BUILD_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) $(CARGO_PROFILE) --no-default-features
export KERNEL_CARGO_CLEAN_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clean $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CHECK_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 check $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) --message-format=json --no-default-features
export KERNEL_CARGO_CLIPPY_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) --no-default-features
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
# Configuration for Tests
#===================================================================================================

export NANVIXD_SOCKADDR := $(if $(filter yes,$(RELEASE)),127.0.0.1:8181,127.0.0.1:8282)

#===================================================================================================
# Top-Level Targets
#===================================================================================================

ALL_GUEST_STATIC_LIBS := posix
ALL_GUEST_RUST_LIBS := arch bitmap config elf error type-safe nvx proc raw-array slab static_assert sysapi syscall sysalloc syslog-macros syslog sys libc_stdlib libc_string
ALL_GUEST_RUST_LIBS_TEST_LIST := arch bitmap config elf error type-safe proc raw-array slab static_assert libc_string syslog-macros syslog

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo-rust-nostd noop-rust-nostd
ALL_GUEST_APPLICATIONS := hello-rust-nostd
ALL_GUEST_TESTS := testd file-rust thread-rust stress-rust linux-app arch-rust
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES += $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := echo-wasm-rust hello-wasm noop-wasm-rust

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
ALL_HOST_RUST_LIBS := control-plane-api hwloc profiler nanvix nanvix-http nanvix-registry nanvix-sandbox nanvix-sandbox-cache nanvix-terminal syscomm user-vm-api
ALL_HOST_UTILS := echo-client
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
all: all-nanvix all-opt
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
all-nanvix: all-host-binaries all-nanvixd all-uservm all-nanvix-bench all-nanvix-test
endif

# Performs local initialization.
init: init-repo init-opt

init-repo:
	$(MKDIR_CMD) $(BINARIES_DIR)
	$(MKDIR_CMD) $(LIBRARIES_DIR)
	$(MKDIR_CMD) $(LOGS_DIR)
	git config --local core.hooksPath .githooks

# Cleans build.
clean: \
	clean-guest-staticlibs \
	clean-guest-binaries \
	clean-wasmd \
	clean-kernel \
	clean-wasm-binaries \
	clean-opt \
	clean-snapshot \
	image-clean

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
clean: clean-host-binaries clean-nanvixd clean-uservm clean-nanvix-bench clean-nanvix-test
endif

distclean: clean
	$(FORCE_RM_CMD) Cargo.lock
	$(FORCE_RM_CMD) $(OBJECTS_DIR)
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
	@cp ${RUN_NANVIXD_SCRIPT} ${SYSROOT_DIR}/etc/scripts/
endif
	@cp ${LIBPOSIX} ${SYSROOT_DIR}/lib/
	@cp -r ${SCRIPTS_DIR}/common/* ${SYSROOT_DIR}/etc/scripts/
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
	@echo ""
	@echo "Testing Targets"
	@echo "  run-unit-tests       Run unit tests for libraries and components"
	@echo "  run-nanvixd-tests    Run system integration tests"
	@echo ""
	@echo "Execution Targets"
	@echo "  debug    Run system in debug mode"
	@echo "  image    Build system image for deployment"
	@echo "  run      Run system in release mode"
	@echo ""
	@echo "Build Parameters (override with VAR=value, see Parameter Values section below)"
	@echo "  BUILD_OPT        Build optional software (default: $(BUILD_OPT)) [impacts build time]"
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
	@echo ""
	@echo "Parameter Values"
	@echo "  MACHINE         hyperlight, microvm, qemu-pc, qemu-isapc, qemu-baremetal"
	@echo "  TARGET          x86"
	@echo "  RELEASE         yes, no"
	@echo "  LOG_LEVEL       trace, debug, info, warn, error"
	@echo "  PROFILER        yes, no"
	@echo "  BUILD_OPT       yes, no"
	@echo "  L2_VM           yes, no"
	@echo "  MAKE_NO_PRINT   yes, no"

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
rust-lint-check: rust-lint-check-host-binaries rust-lint-check-host-rlibs rust-lint-check-nanvixd rust-lint-check-uservm rust-lint-check-nanvix-bench rust-lint-check-nanvix-test
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
rust-lint: rust-lint-host-binaries rust-lint-host-rlibs rust-lint-nanvixd rust-lint-uservm rust-lint-nanvix-bench rust-lint-nanvix-test
endif

# Fixes spelling errors in source code and documentation.
spellcheck-fix:
	codespell --write-changes $(shell git ls-files)

# Checks for spelling errors in source code and documentation.
spellcheck:
	codespell $(shell git ls-files)

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
rust-format: format-host-binaries format-host-rlibs format-nanvixd format-uservm format-nanvix-bench format-nanvix-test
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
rust-format-check: format-check-host-binaries format-check-host-rlibs format-check-nanvixd format-check-uservm format-check-nanvix-bench format-check-nanvix-test
endif

# Python lint variables
PY_VERBOSE :=
ifneq ($(VERBOSE),yes)
PY_VERBOSE += >> /dev/null 2>&1
endif
PYTHON_VENV_DIRECTORY=$(ROOT_DIR)/venv

python-init:
	@if [ ! -f $(PYTHON_VENV_DIRECTORY)/bin/pip3 ]; then \
		$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY); \
		python3 -m venv $(PYTHON_VENV_DIRECTORY); \
	fi
	@$(PYTHON_VENV_DIRECTORY)/bin/pip3 install "black>=24.0.0" "flake8>=7.0.0" > /dev/null

python-format: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m black $(shell git ls-files -- "*.py") $(PY_VERBOSE)

python-format-check: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m black --check $(shell git ls-files -- "*.py") $(PY_VERBOSE)

python-lint: python-init
	@$(PYTHON_VENV_DIRECTORY)/bin/python3 -m flake8 $(shell git ls-files -- "*.py") $(PY_VERBOSE)

# Checks for linting issues in shell scripts.
shell-lint-check:
	@shellcheck -S warning $(shell git ls-files -- "*.sh")

# Fixes code linting issues in shell scripts.
shell-lint:
	@scripts/shell-lint-fix.sh

# Check C/C++ formatting style.
clang-format-check:
	@clang-format --dry-run --Werror $(shell git ls-files -- "*.c" "*.cpp" "*.h" "*.hpp")

# Format C/C++ files.
clang-format:
	@clang-format -i $(shell git ls-files -- "*.c" "*.cpp" "*.h" "*.hpp")

check: \
	check-kernel \
	check-guest-binaries \
	check-guest-rlibs \
	check-guest-staticlibs \
	check-wasmd \
	check-wasm-binaries

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
check: check-host-binaries check-host-rlibs check-nanvixd check-uservm check-nanvix-bench check-nanvix-test
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
	@$(MAKE) run-nanvixd-tests
else
	@echo "Skipping run-nanvixd-tests; MACHINE=$(MACHINE) does not support nanvixd system tests."
endif

run-unit-tests: all-nanvix test-guest-rlibs

ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
run-unit-tests: test-host-rlibs
endif

include build/make/test.mk

#===================================================================================================
# Build Rules for Optional Software
#===================================================================================================

include build/make/optional.mk

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
