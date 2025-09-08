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
export L2_VM ?= yes

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
# Directories
#===================================================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  := $(ROOT_DIR)/bin
export LIBRARIES_DIR := $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build
export IMAGE_DIR     := $(BUILD_DIR)/iso
export SNAPSHOT_DIR  := $(ROOT_DIR)/images
export LOGS_DIR      := $(ROOT_DIR)/logs
export SCRIPTS_DIR   := $(ROOT_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain
export SYSROOT_DIR   ?= $(ROOT_DIR)/sysroot$(if $(filter yes,$(RELEASE)),-release,-debug)
export TARGETS_DIR   := $(BUILD_DIR)/targets
export OBJECTS_DIR   := $(ROOT_DIR)/target
export SCCACHE       ?= $(shell which sccache 2>/dev/null)

#===================================================================================================
# Libraries and Binaries
#===================================================================================================

# File format for executables.
export EXEC_FORMAT := elf

# Libraries
export LIBC := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
export LIBM := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libm.a
export LIBCXX := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libstdc++.a
export LIBNVX := $(LIBRARIES_DIR)/libnvx.a
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a

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
export CC := $(TOOLCHAIN_DIR)/bin/i686-nanvix-gcc
export CXX := $(TOOLCHAIN_DIR)/bin/i686-nanvix-g++

# SCCACHE integration for C/C++ compilation (optional)
ifneq ($(SCCACHE),)
export CC := $(SCCACHE) $(CC)
export CXX := $(SCCACHE) $(CXX)
endif

# C Compiler Options
export CFLAGS := -std=c17
export CFLAGS += -m32 -march=pentiumpro -Wa,-march=pentiumpro
export CFLAGS += -Wall -Wextra -Werror
export CFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export CFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export CFLAGS += -Wvla -Wredundant-decls
export CFLAGS += -pedantic-errors
export CFLAGS += -Wstack-usage=4096
export CFLAGS += -D__NANVIX_SYSNAME__="\"$(NANVIX_SYSNAME)\""
export CFLAGS += -D__NANVIX_NODENAME__="\"$(NANVIX_NODENAME)\""
export CFLAGS += -D__$(subst -,_,$(NANVIX_MACHINE))__

# C++ Compiler Options
export CXXFLAGS := -std=c++17
export CXXFLAGS += -m32 -march=pentiumpro -Wa,-march=pentiumpro
export CXXFLAGS += -Wall -Wextra -Werror
export CXXFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export CXXFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export CXXFLAGS += -Wvla -Wredundant-decls
export CXXFLAGS += -pedantic-errors
export CXXFLAGS += -Wstack-usage=4096
export CXXFLAGS += -D__NANVIX_SYSNAME__="\"$(NANVIX_SYSNAME)\""
export CXXFLAGS += -D__NANVIX_NODENAME__="\"$(NANVIX_NODENAME)\""
export CXXFLAGS += -D__$(subst -,_,$(NANVIX_MACHINE))__

# Linker Options
export LDFLAGS := -z noexecstack -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

# Optimization Flags
ifeq ($(RELEASE), yes)
export CFLAGS += -O3
export CXXFLAGS += -O3
export CFLAGS += -D__RELEASE
export CXXFLAGS += -D__RELEASE
else
export CFLAGS += -O0
export CFLAGS += -g
export CXXFLAGS += -O0
export CFLAGS += -D__DEBUG
export CXXFLAGS += -D__DEBUG
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
export KERNEL_CARGO_FEATURES := --no-default-features --features $(MACHINE) --features $(LOG_LEVEL)
export WASMD_CARGO_FEATURES :=

# Rust flags for host target.
export HOST_RUST_FLAGS := $(if $(HOST_CPU),-C target-cpu=$(HOST_CPU))
export HOST_CARGO_FEATURES := --no-default-features
export HOST_CARGO_FEATURES += $(if $(filter yes,$(TIMESTAMP_MSG)),--features timestamp-messages,)
export MICROVM_CARGO_FEATURES := --no-default-features
export MICROVM_CARGO_FEATURES += $(if $(filter yes,$(PROFILER)),--features profiler,)
export MICROVM_CARGO_FEATURES += $(if $(filter yes,$(TIMESTAMP_MSG)),--features timestamp-messages,)
export MICROVM_CARGO_FEATURES += $(if $(filter hyperlight,$(MACHINE)),--features hyperlight,)

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
# JavaScript Toolchain Configuration
#===================================================================================================

# Tools
export JAVY ?= $(HOME)/.cargo/bin/javy

# Javy compiler options.
export JAVY_FLAGS := -J simd-json-builtins=n -C dynamic=no -C source-compression=y

#===================================================================================================
# Commands
#===================================================================================================

# Cargo commands for guest target.
export GUEST_CARGO_BUILD_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) $(CARGO_PROFILE)
export GUEST_CARGO_CLEAN_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 clean $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CHECK_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 check $(GUEST_CARGO_FLAGS)  $(GUEST_CARGO_TARGET) --message-format=json
export GUEST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_FMT_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

export KERNEL_CARGO_BUILD_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) $(CARGO_PROFILE)
export KERNEL_CARGO_CLEAN_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clean $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CHECK_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 check $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) --message-format=json
export KERNEL_CARGO_CLIPPY_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_FMT_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

# Cargo commands for wasm target.
export WASM_CARGO_BUILD_CMD := $(CARGO) +nanvix-x86 build $(WASM_CARGO_PROFILE) --target wasm32-wasip1
export WASM_CARGO_CLEAN_CMD := $(CARGO) +nanvix-x86 clean --target wasm32-wasip1
export WASM_CARGO_CHECK_CMD := $(CARGO) +nanvix-x86 check --target wasm32-wasip1 --message-format=json
export WASM_CARGO_CLIPPY_CMD := $(CARGO) +nanvix-x86 clippy --target wasm32-wasip1
export WASM_CARGO_FMT_CMD := $(CARGO) +nanvix-x86 fmt

# Cargo commands for host target.
export HOST_CARGO_BUILD_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 build $(CARGO_PROFILE)
export HOST_CARGO_CLEAN_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 clean
export HOST_CARGO_CHECK_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 check --message-format=json
export HOST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 clippy
export HOST_CARGO_TEST_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 test --no-default-features --features=std
export HOST_CARGO_FMT_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) +nanvix-x86 fmt

# Utility Commands
export RM_CMD := rm -f
export FORCE_RM_CMD := rm -rf
export MKDIR_CMD := mkdir -p
export CP_CMD := cp -f --preserve
export GRUB_CMD := grub-mkrescue

#===================================================================================================
# Configuration for Tests
#===================================================================================================

export NANVIXD_SOCKADDR := $(if $(filter yes,$(RELEASE)),127.0.0.1:8181,127.0.0.1:8282)

#===================================================================================================
# Top-Level Targets
#===================================================================================================

ALL_GUEST_STATIC_LIBS := posix
ALL_GUEST_RUST_LIBS := arch bitmap config elf error type-safe nvx proc raw-array slab static_assert sysapi syscall sysalloc syslog sys

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo-rust-nostd noop-rust-nostd
ALL_GUEST_APPLICATIONS := hello-rust-nostd
ALL_GUEST_TESTS := testd file-rust linux-app arch-rust
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES +=  $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := echo-wasm-rust hello-wasm noop-wasm-rust

ALL_HOST_RUST_LIBS := hwloc profiler syscomm
ALL_HOST_UTILS := echo-client nanvix-bench nanvixd
ALL_HOST_DAEMONS := linuxd
ALL_HOST_BINARIES := $(ALL_HOST_UTILS) $(MICROVM) $(ALL_HOST_DAEMONS)

#===================================================================================================
# Top-Level Build Rules
#===================================================================================================

# Builds everything.
all: all-nanvix all-opt

# Builds all Nanvix components.
all-nanvix: \
	init \
	all-guest-staticlibs \
	all-guest-binaries \
	all-wasmd \
	all-kernel \
	all-wasm-binaries \
	all-host-binaries \
	all-microvm \
	all-snapshot

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
	clean-host-binaries \
	clean-microvm \
	clean-opt \
	clean-snapshot \
	image-clean

distclean: clean
	$(FORCE_RM_CMD) Cargo.lock
	$(FORCE_RM_CMD) $(OBJECTS_DIR)
	$(FORCE_RM_CMD) $(LIBRARIES_DIR)
	$(FORCE_RM_CMD) $(BINARIES_DIR)
	$(FORCE_RM_CMD) $(PYTHON_VENV_DIRECTORY)

# Installs build artifacts.
install: all-nanvix
	@echo "Installing Nanvix in ${SYSROOT_DIR}..."
	@mkdir -p ${SYSROOT_DIR}/bin
	@mkdir -p ${SYSROOT_DIR}/lib
	@mkdir -p ${SYSROOT_DIR}/etc/scripts
	@cp -r ${BINARIES_DIR}/* ${SYSROOT_DIR}/bin/
	@cp -r ${LIBRARIES_DIR}/* ${SYSROOT_DIR}/lib/
	@cp -r ${SCRIPTS_DIR}/common/* ${SYSROOT_DIR}/etc/scripts/
	@cp -r ${BUILD_DIR}/user/linker/$(TARGET)/user.ld ${SYSROOT_DIR}/lib/

# Shows available make targets and build parameters.
help:
	@echo ""
	@echo "Main Build Targets"
	@echo "  all          Build everything (default target)"
	@echo "  clean        Remove build artifacts and intermediate files"
	@echo "  distclean    Cleans everything"
	@echo "  help         Show this help message"
	@echo ""
	@echo "Development Targets"
	@echo "  check           Run all validation checks (syntax, compilation)"
	@echo "  format          Fix code formatting issues automatically"
	@echo "  format-check    Check code formatting without fixing"
	@echo "  install         Install build artifacts in the sysroot directory"
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
	@echo "  TARGET           Target architecture (default: $(TARGET))"
	@echo "  MACHINE          Target machine type (default: $(MACHINE))"
	@echo "  RELEASE          Release build mode (default: $(RELEASE)) [impacts build time]"
	@echo "  LOG_LEVEL        Logging verbosity (default: $(LOG_LEVEL))"
	@echo "  BUILD_OPT        Build optional software (default: $(BUILD_OPT)) [impacts build time]"
	@echo "  TIMEOUT          Execution timeout in seconds (default: $(TIMEOUT))"
	@echo "  TOOLCHAIN_DIR    Toolchain location (default: $(TOOLCHAIN_DIR))"
	@echo "  PROFILER         Enable MicroVM profiler (default: $(PROFILER))"
	@echo "  JAVY             Javy compiler location (default: $(JAVY)) [impacts build time]"
	@echo "  SCCACHE          Path to compilation cache binary (default: auto-detected from PATH) [impacts build time]"
	@echo "  L2_VM            Enable L2 VM deployment (default: $(L2_VM))"
	@echo "  SYSROOT_DIR      Sysroot directory (default: $(SYSROOT_DIR))"
	@echo ""
	@echo "Parameter Values"
	@echo "  MACHINE      hyperlight, microvm, qemu-pc, qemu-isapc, qemu-baremetal"
	@echo "  TARGET       x86"
	@echo "  RELEASE      yes, no"
	@echo "  LOG_LEVEL    trace, debug, info, warn, error"
	@echo "  PROFILER     yes, no"
	@echo "  BUILD_OPT    yes, no"
	@echo "  JAVY         path to javy executable"
	@echo "  L2_VM        yes, no"

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
	rust-lint-check-wasm-binaries \
	rust-lint-check-host-binaries \
	rust-lint-check-host-rlibs \
	rust-lint-check-microvm

# Fixes code linting issues.
rust-lint: \
	rust-lint-kernel \
	rust-lint-guest-binaries \
	rust-lint-guest-rlibs \
	rust-lint-guest-staticlibs \
	rust-lint-wasmd \
	rust-lint-wasm-binaries \
	rust-lint-host-binaries \
	rust-lint-host-rlibs \
	rust-lint-microvm

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
	format-host-binaries \
	format-host-rlibs \
	format-kernel \
	format-microvm \
	format-wasmd \
	format-wasm-binaries

# Checks Rust code formatting.
rust-format-check: \
	format-check-guest-binaries \
	format-check-guest-rlibs \
	format-check-guest-staticlibs \
	format-check-host-binaries \
	format-check-host-rlibs \
	format-check-kernel \
	format-check-microvm \
	format-check-wasmd \
	format-check-wasm-binaries

# Python lint variables
PY_VERBOSE :=
ifneq ($(VERBOSE),yes)
PY_VERBOSE += >> /dev/null 2>&1
endif
PYTHON_VENV_DIRECTORY=$(ROOT_DIR)/venv

python-init:
	@if [ ! -d $(PYTHON_VENV_DIRECTORY) ]; then python3 -m venv $(PYTHON_VENV_DIRECTORY); fi
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
	check-wasm-binaries \
	check-host-binaries \
	check-host-rlibs \
	check-microvm

run-unit-tests: all-nanvix \
	test-guest-rlibs

run-nanvixd-tests: | \
	init-repo \
	test-dlfcn-c \
	test-echo-c \
	test-echo-cpp \
	test-echo-rust-nostd \
	test-echo-wasm-js \
	test-echo-wasm-rust \
	test-file-c \
	test-file-rust \
	test-hello-c \
	test-hello-cpp \
	test-hello-js \
	test-hello-wasm \
	test-linux-app \
	test-memory-c \
	test-misc-c \
	test-network-c \
	test-python3 \
	test-arch-rust \
	test-thread-c

#===================================================================================================
# Build Rules for Optional Software
#===================================================================================================

ifneq ($(strip $(filter yes,$(BUILD_OPT))),)

all-opt: init all-openblas all-openssl all-python all-sqlite all-zlib

clean-opt: clean-openblas clean-openssl clean-python clean-sqlite clean-zlib

init-opt: init-openblas init-openssl init-python init-sqlite init-zlib

else

all-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
clean-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
init-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"

endif

#===================================================================================================
# Build Rules for OpenBLAS
#===================================================================================================

OPENBLAS_LIB := $(SYSROOT_DIR)/lib/libopenblas.a

all-openblas: $(OPENBLAS_LIB)

$(OPENBLAS_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@if [ ! -f $@ ]; then \
		echo "Building OpenBLAS (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-openblas.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building OpenBLAS (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-openblas.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	else \
		echo "OpenBLAS up-to-date!"; \
	fi
endif

clean-openblas: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openblas.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
	$(RM_CMD) $(OPENBLAS_LIB)
endif

init-openblas: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openblas.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for OpenSSL
#===================================================================================================

CRYPTO_LIB := $(SYSROOT_DIR)/lib/libcrypto.a
OPENSSL_LIB := $(SYSROOT_DIR)/lib/libssl.a

all-openssl: $(OPENSSL_LIB)

$(OPENSSL_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@if [ ! -f $@ ]; then \
		echo "Building OpenSSL (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-openssl.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building OpenSSL (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-openssl.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	else \
		echo "OpenSSL up-to-date!"; \
	fi
endif

clean-openssl:
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openssl.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
	$(RM_CMD) $(OPENSSL_LIB) $(CRYPTO_LIB)
endif

init-openssl: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openssl.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Python
#===================================================================================================

PYTHON_LIB := $(SYSROOT_DIR)/lib/libpython3.12.a

all-python: $(PYTHON_LIB)

$(PYTHON_LIB): init-repo install all-openssl all-sqlite all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@if [ ! -f $@ ]; then \
		echo "Building Python (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-python.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building Python (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-python.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	else \
		echo "Python up-to-date!"; \
	fi
endif

clean-python: clean-sqlite clean-openssl clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-python.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
	$(RM_CMD) $(PYTHON_LIB)
endif

init-python: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-python.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Sqlite
#===================================================================================================

SQLITE_LIB := $(SYSROOT_DIR)/lib/libsqlite3.a

all-sqlite: $(SQLITE_LIB)

$(SQLITE_LIB): init-repo install all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@if [ ! -f $@ ]; then \
		echo "Building SQLite (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-sqlite.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building SQLite (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-sqlite.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	else \
		echo "SQLite up-to-date!"; \
	fi
endif

clean-sqlite: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-sqlite.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
	$(RM_CMD) $(SQLITE_LIB)
endif

init-sqlite: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-sqlite.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Zlib
#===================================================================================================

ZLIB_LIB := $(SYSROOT_DIR)/lib/libz.a

all-zlib: $(ZLIB_LIB)

$(ZLIB_LIB): init-repo install
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@if [ ! -f $@ ]; then \
		echo "Building ZLib (missing) ..."; \
		bash $(SCRIPTS_DIR)/build-zlib.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	elif [ $@ -ot $(LIBPOSIX) ]; then \
		echo "Building ZLib (outdated) ..."; \
		bash $(SCRIPTS_DIR)/build-zlib.sh build $(TOOLCHAIN_DIR) $(SYSROOT_DIR); \
	else \
		echo "ZLib up-to-date!"; \
	fi
endif

clean-zlib:
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-zlib.sh clean $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
	$(RM_CMD) $(ZLIB_LIB)
endif

init-zlib: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-zlib.sh init $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Running and Debugging
#===================================================================================================

# Runs system in release mode.
run: image
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)
else
	sudo -E $(BINARIES_DIR)/microvm.elf -user-vm-id 1 -kernel $(BINARIES_DIR)/kernel.elf -initrd $(IMAGE) 2>&1
endif

# Runs system in debug mode.
debug: image
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --debug $(TIMEOUT)
endif

#===================================================================================================
# Build Rules for System Image
#===================================================================================================

# Builds the system image.
image: all-nanvix
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	$(CP_CMD) $(BINARIES_DIR)/*.$(EXEC_FORMAT) $(IMAGE_DIR)/
	$(GRUB_CMD) $(IMAGE_DIR) -o $(IMAGE)
endif

image-clean:
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	$(RM_CMD) $(IMAGE_DIR)/*.$(EXEC_FORMAT)
	$(RM_CMD) $(IMAGE)
endif

#===================================================================================================
# Build Rules for L2 System VM Snapshot
#===================================================================================================

# The snapshots for the L2 VM need linuxd.elf to be built first.
all-snapshot: all-host-binaries
# Snapshots are only generated for microvm/hyperlight machines when L2_VM is enabled.
ifneq (,$(and $(filter yes,$(L2_VM)),$(filter $(MACHINE),microvm hyperlight)))
	bash $(SCRIPTS_DIR)/generate-l2-initramfs.sh
	bash $(SCRIPTS_DIR)/generate-l2-snapshot.sh $(TOOLCHAIN_DIR)
endif

clean-snapshot:
	$(FORCE_RM_CMD) $(SNAPSHOT_DIR)

#===================================================================================================
# Build Rules for Guest Static Libraries
#===================================================================================================

define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$(1).a $(LIBRARIES_DIR)/lib$(1).a

check-guest-staticlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
#	$(HOST_CARGO_CHECK_CMD) -p $(1) --no-default-features --features=std --all-targets

format-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-staticlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/lib$(1).a

rust-lint-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL) --fix --allow-dirty
#	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets --fix --allow-dirty

rust-lint-check-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
#	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets
endef

$(foreach target,$(ALL_GUEST_STATIC_LIBS),$(eval $(call GUEST_STATICLIB_RULES,$(target))))

all-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),all-guest-staticlib-$(target))

check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),check-guest-staticlib-$(target))

format-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),format-guest-staticlib-$(target))

format-check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),format-check-guest-staticlib-$(target))

clean-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clean-guest-staticlib-$(target))

rust-lint-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),rust-lint-guest-staticlib-$(target))

rust-lint-check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),rust-lint-check-guest-staticlib-$(target))

#===================================================================================================
# Build Rules for Guest Rust Libraries
#===================================================================================================

define GUEST_RLIB_RULES
check-guest-rlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
#	$(HOST_CARGO_CHECK_CMD) -p $(1) --no-default-features --features=std --all-targets

format-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-rlib-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty
#	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets --fix --allow-dirty

rust-lint-check-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1)
#	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS),$(eval $(call GUEST_RLIB_RULES,$(target))))

check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),check-guest-rlib-$(target))

format-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),format-guest-rlib-$(target))

format-check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),format-check-guest-rlib-$(target))

rust-lint-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),rust-lint-guest-rlib-$(target))

rust-lint-check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),rust-lint-check-guest-rlib-$(target))

test-guest-rlibs:
	$(HOST_CARGO_TEST_CMD) -p arch
	$(HOST_CARGO_TEST_CMD) -p bitmap
	$(HOST_CARGO_TEST_CMD) -p config
	$(HOST_CARGO_TEST_CMD) -p elf
	$(HOST_CARGO_TEST_CMD) -p error
	$(HOST_CARGO_TEST_CMD) -p type-safe
	$(HOST_CARGO_TEST_CMD) -p proc
	$(HOST_CARGO_TEST_CMD) -p raw-array
	$(HOST_CARGO_TEST_CMD) -p slab
	$(HOST_CARGO_TEST_CMD) -p static_assert
#	$(HOST_CARGO_TEST_CMD) -p sysalloc
#	$(HOST_CARGO_TEST_CMD) -p syslog

#===================================================================================================
# Build Rules for Guest Binaries
#===================================================================================================

define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=$(LOG_LEVEL)

format-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1)

format-check-guest-binaries-$(1):
	$(GUEST_CARGO_FMT_CMD) -p $(1) --check

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=$(LOG_LEVEL) --fix --allow-dirty
#	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets --fix --allow-dirty

rust-lint-check-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=$(LOG_LEVEL)
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

all-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),all-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks all
	$(MAKE) -C $(SOURCES_DIR)/user all
	$(MAKE) -C $(SOURCES_DIR)/tests all

check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),check-guest-binaries-$(target))

format-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-guest-binaries-$(target))

format-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),format-check-guest-binaries-$(target))

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks clean
	$(MAKE) -C $(SOURCES_DIR)/user clean
	$(MAKE) -C $(SOURCES_DIR)/tests clean

rust-lint-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-guest-binaries-$(target))

rust-lint-check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),rust-lint-check-guest-binaries-$(target))

all-wasmd: all-wasm-binaries all-guest-binaries
	@echo "WASM_BINARY=$(WASM_BINARY)"
ifneq ($(WASM_BINARY),)
	$(eval export NANVIX_WASM_BINARY := $(realpath $(WASM_BINARY)))
	$(eval export NANVIX_WASM_BINARY_BASENAME := $(shell basename $(NANVIX_WASM_BINARY)))
	$(eval export NANVIX_WASM_BINARY_ARGS := =$(WASM_BINARY_ARGS))
	$(eval export WASMD_CARGO_FEATURES := --features wasm_binary)
endif
	@echo "NANVIX_WASM_BINARY=$(NANVIX_WASM_BINARY)"
	@echo "NANVIX_WASM_BINARY_BASENAME=$(NANVIX_WASM_BINARY_BASENAME)"
	@echo "NANVIX_WASM_BINARY_ARGS=$(NANVIX_WASM_BINARY_ARGS)"
	$(GUEST_CARGO_BUILD_CMD) $(WASMD_CARGO_FEATURES) -p wasmd
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/wasmd.elf $(BINARIES_DIR)/wasmd.elf

check-wasmd:
	$(GUEST_CARGO_CHECK_CMD) -p wasmd

format-wasmd:
	$(GUEST_CARGO_FMT_CMD) -p wasmd

format-check-wasmd:
	$(GUEST_CARGO_FMT_CMD) -p wasmd --check

clean-wasmd: clean-wasm-binaries clean-guest-binaries
	$(GUEST_CARGO_CLEAN_CMD) -p wasmd
	$(RM_CMD) $(BINARIES_DIR)/wasmd.elf

rust-lint-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd --fix --allow-dirty

rust-lint-check-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd

#===================================================================================================
# Build Rules for Kernel Binary
#===================================================================================================

all-kernel: init
	$(KERNEL_CARGO_BUILD_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-kernel/$(BUILD_MODE)/kernel.elf $(BINARIES_DIR)/kernel.elf

check-kernel:
	$(KERNEL_CARGO_CHECK_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel

format-kernel:
	$(KERNEL_CARGO_FMT_CMD) -p kernel

format-check-kernel:
	$(KERNEL_CARGO_FMT_CMD) -p kernel --check

clean-kernel:
	$(KERNEL_CARGO_CLEAN_CMD) -p kernel
	$(RM_CMD) $(BINARIES_DIR)/kernel.elf

rust-lint-kernel:
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel --fix --allow-dirty

rust-lint-check-kernel:
	$(KERNEL_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel

#===================================================================================================
# Build Rules for WASM Binaries
#===================================================================================================

define WASM_BINARY_RULES
all-wasm-binaries-$(1): init
	$(WASM_CARGO_BUILD_CMD) -p $(1)
	$(CP_CMD) $(OBJECTS_DIR)/wasm32-wasip1/$(WASM_BUILD_MODE)/$(1).wasm $(BINARIES_DIR)/$(1).wasm

check-wasm-binaries-$(1):
	$(WASM_CARGO_CHECK_CMD) -p $(1)

format-wasm-binaries-$(1):
	$(WASM_CARGO_FMT_CMD) -p $(1)

format-check-wasm-binaries-$(1):
	$(WASM_CARGO_FMT_CMD) -p $(1) --check

clean-wasm-binaries-$(1):
	$(WASM_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).wasm

rust-lint-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty

rust-lint-check-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1)
endef

$(foreach target,$(ALL_WASM_BINARIES),$(eval $(call WASM_BINARY_RULES,$(target))))

all-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),all-wasm-binaries-$(target))

check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),check-wasm-binaries-$(target))

format-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),format-wasm-binaries-$(target))

format-check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),format-check-wasm-binaries-$(target))

clean-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),clean-wasm-binaries-$(target))

rust-lint-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),rust-lint-wasm-binaries-$(target))

rust-lint-check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),rust-lint-check-wasm-binaries-$(target))

#===================================================================================================
# Build Rules for Host Rust Libraries
#===================================================================================================

define HOST_RLIB_RULES
check-host-rlib-$(1):
	$(HOST_CARGO_CHECK_CMD) -p $(1)

format-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-rlib-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

rust-lint-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty

rust-lint-check-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1)

test-host-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1)
endef

$(foreach target,$(ALL_HOST_RUST_LIBS),$(eval $(call HOST_RLIB_RULES,$(target))))

check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),check-host-rlib-$(target))

format-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-host-rlib-$(target))

format-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),format-check-host-rlib-$(target))

rust-lint-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-host-rlib-$(target))

rust-lint-check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),rust-lint-check-host-rlib-$(target))

test-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),test-host-rlib-$(target))

#===================================================================================================
# Build Rules for Host Binaries
#===================================================================================================

define HOST_BINARY_RULES
all-host-binaries-$(1): init
ifeq ($(filter $(1),linuxd nanvix-bench),$(1))
	$(HOST_CARGO_BUILD_CMD) $(HOST_CARGO_FEATURES) -p $(1)
else
	$(HOST_CARGO_BUILD_CMD) -p $(1)
endif
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(1) $(BINARIES_DIR)/$(1).elf

check-host-binaries-$(1):
ifeq ($(filter $(1),linuxd nanvix-bench),$(1))
	$(HOST_CARGO_CHECK_CMD) $(HOST_CARGO_FEATURES) -p $(1)
else
	$(HOST_CARGO_CHECK_CMD) -p $(1)
endif

format-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1)

format-check-host-binaries-$(1):
	$(HOST_CARGO_FMT_CMD) -p $(1) --check

clean-host-binaries-$(1):
	$(HOST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

rust-lint-host-binaries-$(1):
ifeq ($(filter $(1),linuxd nanvix-bench),$(1))
	$(HOST_CARGO_CLIPPY_CMD) $(HOST_CARGO_FEATURES) -p $(1) --fix --allow-dirty
else
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty
endif

rust-lint-check-host-binaries-$(1):
ifeq ($(filter $(1),linuxd nanvix-bench),$(1))
	$(HOST_CARGO_CLIPPY_CMD) $(HOST_CARGO_FEATURES) -p $(1)
else
	$(HOST_CARGO_CLIPPY_CMD) -p $(1)
endif
endef

$(foreach target,$(ALL_HOST_BINARIES),$(eval $(call HOST_BINARY_RULES,$(target))))

all-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),all-host-binaries-$(target))

check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),check-host-binaries-$(target))

format-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),format-host-binaries-$(target))

format-check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),format-check-host-binaries-$(target))

clean-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),clean-host-binaries-$(target))

rust-lint-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),rust-lint-host-binaries-$(target))

rust-lint-check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),rust-lint-check-host-binaries-$(target))

#===================================================================================================
# Build Rules for Microvm Binary
#===================================================================================================

all-microvm: init
	$(HOST_CARGO_BUILD_CMD) $(MICROVM_CARGO_FEATURES) -p microvm
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/microvm $(BINARIES_DIR)/microvm.elf

check-microvm:
	$(HOST_CARGO_CHECK_CMD) $(MICROVM_CARGO_FEATURES) -p microvm

format-microvm:
	$(HOST_CARGO_FMT_CMD) -p microvm

format-check-microvm:
	$(HOST_CARGO_FMT_CMD) -p microvm --check

clean-microvm:
	$(HOST_CARGO_CLEAN_CMD) -p microvm
	$(RM_CMD) $(BINARIES_DIR)/microvm.elf

rust-lint-microvm:
	$(HOST_CARGO_CLIPPY_CMD) $(MICROVM_CARGO_FEATURES) -p microvm --fix --allow-dirty

rust-lint-check-microvm:
	$(HOST_CARGO_CLIPPY_CMD) $(MICROVM_CARGO_FEATURES) -p microvm

#===================================================================================================
# Rules for Running System Level Tests Using Nanvix Daemon
#===================================================================================================

# List of supported functions in hyperlight.
HYPERLIGHT_WHITELIST := echo-c echo-cpp echo-rust-nostd hello-c hello-cpp

comma:=,

define TEST_RULE
test-$(2): all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(2)..."
ifeq ($(MACHINE),hyperlight)
	@if echo "$(HYPERLIGHT_WHITELIST)" | grep -wq "$(2)"; then \
		if [ `stat -c%s "$(1)/$(2)$(3)"` -gt 16777216 ]; then \
			echo "\033[31mWarning: $(1)/$(2)$(3) exceeds 16 MB, skipping test.\033[0m"; \
		else \
			$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(1)/$(2)$(3) $(4) $(5) $(6) $(TIMEOUT); \
		fi; \
	else \
		echo "\033[31mWarning: Skipping $(2) on hyperlight (not supported).\033[0m"; \
	fi
else
	$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(1)/$(2)$(3) $(4) $(5) $(6) $(TIMEOUT)
endif
endif
endef

$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-c,.elf,'','["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-cpp,.elf,'','["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-rust-nostd,.elf,'','["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),hello-c,.elf,'','[]','Hello$(comma) world from C!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),hello-cpp,.elf,'','[]','Hello$(comma) world from C++!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),linux-app,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),dlfcn-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),file-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),file-rust,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),thread-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),network-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),misc-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),memory-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),arch-rust,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(SYSROOT_DIR)/bin,python3,,'$(SOURCES_DIR)/user/hello-python/__main__.py','','Hello$(comma) from Python!'))

define WASM_TEST_RULE
test-$(1): all
ifeq ($(shell basename $(WASM_BINARY)),$(1).wasm)
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(1)..."
ifeq ($(MACHINE),hyperlight)
	if [ `stat -c%s "bin/wasmd.elf"` -gt 16777216 ]; then \
		echo "\033[31mWarning: bin/wasmd.elf exceeds 16 MB, skipping test!\033[0m"; \
	else \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) bin/wasmd.elf $(2) $(3) $(4) $(TIMEOUT); \
	fi
else
	$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) bin/wasmd.elf $(2) $(3) $(4) $(TIMEOUT)
endif
endif
endif
endef

$(eval $(call WASM_TEST_RULE,echo-wasm-js,'','["hello world!"]','hello world!'))
$(eval $(call WASM_TEST_RULE,echo-wasm-rust,'','["hello world!"]','hello world!'))
$(eval $(call WASM_TEST_RULE,hello-js,'','[]','Hello$(comma) world from JavaScript!'))
$(eval $(call WASM_TEST_RULE,hello-wasm,'','[]','Hello$(comma) world!'))
