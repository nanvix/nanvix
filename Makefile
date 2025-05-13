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
export TIMEOUT ?= 90

# Enable Microvm profiler?
export PROFILER ?= no

# Target Host CPU
export HOST_CPU ?=

# Build optional software?
export BUILD_OPT ?= yes

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
export SCRIPTS_DIR   := $(ROOT_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain
export SYSROOT_DIR   ?= $(ROOT_DIR)/sysroot$(if $(filter yes,$(RELEASE)),-release,-debug)
export TARGETS_DIR   := $(BUILD_DIR)/targets
export OBJECTS_DIR   := $(ROOT_DIR)/target

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

# Rust flags for guest target.
export GUEST_RUST_FLAGS := "-C relocation-model=static -C prefer-dynamic=no"
export GUEST_CARGO_FLAGS := -Zbuild-std=core,alloc
export GUEST_CARGO_TARGET := --target $(TARGETS_DIR)/$(TARGET)-user.json
export KERNEL_RUST_FLAGS := $(GUEST_RUST_FLAGS)
export KERNEL_CARGO_FLAGS := -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
export KERNEL_CARGO_TARGET := --target $(TARGETS_DIR)/$(TARGET)-kernel.json
export KERNEL_CARGO_FEATURES := --no-default-features --features $(MACHINE) --features $(LOG_LEVEL)
export WASMD_CARGO_FEATURES :=

# Rust flags for host target.
export HOST_RUST_FLAGS := $(if $(HOST_CPU),-C target-cpu=$(HOST_CPU))
export MICROVM_CARGO_FEATURES := --no-default-features
export MICROVM_CARGO_FEATURES += $(if $(filter yes,$(PROFILER)),--features profiler,)
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
export GUEST_CARGO_BUILD_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) build $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET) $(CARGO_PROFILE)
export GUEST_CARGO_CLEAN_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clean $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CHECK_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) check $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET) --message-format=json
export GUEST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clippy $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)

export KERNEL_CARGO_BUILD_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) build $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) $(CARGO_PROFILE)
export KERNEL_CARGO_CLEAN_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) clean $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)
export KERNEL_CARGO_CHECK_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) check $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET) --message-format=json
export KERNEL_CARGO_CLIPPY_CMD := RUSTFLAGS=$(KERNEL_RUST_FLAGS) $(CARGO) clippy $(KERNEL_CARGO_FLAGS) $(KERNEL_CARGO_TARGET)

# Cargo commands for wasm target.
export WASM_CARGO_BUILD_CMD := $(CARGO) build $(WASM_CARGO_PROFILE) --target wasm32-wasip1
export WASM_CARGO_CLEAN_CMD := $(CARGO) clean --target wasm32-wasip1
export WASM_CARGO_CHECK_CMD := $(CARGO) check --target wasm32-wasip1 --message-format=json
export WASM_CARGO_CLIPPY_CMD := $(CARGO) clippy --target wasm32-wasip1

# Cargo commands for host target.
export HOST_CARGO_BUILD_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) build $(CARGO_PROFILE)
export HOST_CARGO_CLEAN_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) clean
export HOST_CARGO_CHECK_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) check --message-format=json
export HOST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) clippy
export HOST_CARGO_TEST_CMD := RUSTFLAGS=$(HOST_RUST_FLAGS) $(CARGO) test --no-default-features --features=std

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
export LINUXD_SOCKADDR := $(if $(filter yes,$(RELEASE)),127.0.0.1:7171,127.0.0.1:7272)
export SANDBOX_SOCKADDR := $(if $(filter yes,$(RELEASE)),127.0.0.1:6161,127.0.0.1:6262)

#===================================================================================================
# Top-Level Targets
#===================================================================================================

ALL_GUEST_STATIC_LIBS := nvx posix
ALL_GUEST_RUST_LIBS := bitmap config elf error type-safe proc raw-array slab static_assert sys time

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo-rust-nostd noop-rust-nostd matmul
ALL_GUEST_APPLICATIONS := hello-rust-nostd
ALL_GUEST_TESTS := testd linux-app
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES +=  $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := echo-wasm-rust hello-wasm noop-wasm-rust

ALL_HOST_RUST_LIBS := profiler
ALL_HOST_UTILS := echo-client loader nanvixd
ALL_HOST_DAEMONS := linuxd
ALL_HOST_BINARIES := $(ALL_HOST_UTILS) $(MICROVM) $(ALL_HOST_DAEMONS)

#===================================================================================================
# Top-Level Build Rules
#===================================================================================================

# Builds everything.
all: \
	init \
	all-guest-staticlibs \
	all-guest-binaries \
	all-wasmd \
	all-kernel \
	all-wasm-binaries \
	all-host-binaries \
	all-microvm \
	all-opt

# Performs local initialization.
init: init-repo init-opt

init-repo:
	$(MKDIR_CMD) $(BINARIES_DIR)
	$(MKDIR_CMD) $(LIBRARIES_DIR)
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
	image-clean

distclean: clean distclean-opt
	$(FORCE_RM_CMD) Cargo.lock
	$(FORCE_RM_CMD) $(OBJECTS_DIR)
	$(FORCE_RM_CMD) $(LIBRARIES_DIR)
	$(FORCE_RM_CMD) $(BINARIES_DIR)

# Runs clippy.
clippy: \
	clippy-kernel \
	clippy-guest-binaries \
	clippy-guest-rlibs \
	clippy-guest-staticlibs \
	clippy-wasmd \
	clippy-wasm-binaries \
	clippy-host-binaries \
	clippy-host-rlibs \
	clippy-microvm

# Python lint variables
PY_CHECK :=
ifeq ($(check),true)
PY_CHECK += --check
endif
PY_VERBOSE :=
ifneq ($(VERBOSE),yes)
PY_VERBOSE += >> /dev/null 2>&1
endif

python-lint:
	@rm -rf /tmp/venv
	@python3 -m venv /tmp/venv
	@/tmp/venv/bin/pip3 install "black>=24.0.0" "flake8>=7.0.0" > /dev/null
	@/tmp/venv/bin/python3 -m black $(PY_CHECK) $(shell git ls-files -- "*.py") $(PY_VERBOSE)
	@/tmp/venv/bin/python3 -m flake8 $(shell git ls-files -- "*.py") $(PY_VERBOSE)
	@rm -rf /tmp/venv

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

run-unit-tests: all \
	test-guest-staticlibs \
	test-guest-rlibs

run-nanvixd-tests: | \
	test-echo-c \
	test-echo-cpp \
	test-echo-rust-nostd \
	test-echo-wasm-js \
	test-echo-wasm-rust \
	test-hello-c \
	test-hello-cpp \
	test-hello-js \
	test-hello-wasm \
	test-linux-app \
	test-dlfcn-c \
	test-file-c \
	test-thread-c \
	test-memory-c \
	test-misc-c \
	test-network-c

# TODO: enable wasm tests, enable thread test.
run-linuxd-tests: | \
	test-linuxd-echo-c \
	test-linuxd-echo-cpp \
	test-linuxd-echo-rust-nostd \
	test-linuxd-hello-c \
	test-linuxd-hello-cpp \
	test-linuxd-linux-app \
	test-linuxd-dlfcn-c \
	test-linuxd-file-c \
	test-linuxd-memory-c \
	test-linuxd-misc-c \
	test-linuxd-network-c \
	test-linuxd-python3

#===================================================================================================
# Build Rules for Optional Software
#===================================================================================================

ifneq ($(strip $(filter yes,$(BUILD_OPT))),)

all-opt: init all-openblas all-python all-sqlite all-zlib

clean-opt: clean-openblas clean-python clean-sqlite clean-zlib

distclean-opt: distclean-openblas distclean-python distclean-sqlite distclean-zlib
	$(FORCE_RM_CMD) $(SYSROOT_DIR)

init-opt: init-openblas init-python init-zlib

else

all-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
clean-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
distclean-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"
init-opt:
	@echo "\033[31mOptional software build disabled. Set BUILD_OPT=yes to build optional software.\033[0m"

endif

#===================================================================================================
# Build Rules for OpenBLAS
#===================================================================================================

all-openblas: init all-guest-staticlibs
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	echo "Building OpenBLAS..."
	bash $(SCRIPTS_DIR)/build-openblas.sh build $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

clean-openblas: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openblas.sh clean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

distclean-openblas: distclean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openblas.sh distclean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

init-openblas: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-openblas.sh init $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Python
#===================================================================================================

all-python: init all-guest-staticlibs all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	echo "Building Python..."
	bash $(SCRIPTS_DIR)/build-python.sh build $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

clean-python: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-python.sh clean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

distclean-python: distclean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-python.sh distclean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

init-python: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-python.sh init $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Sqlite
#===================================================================================================

all-sqlite: init all-guest-staticlibs all-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	echo "Building sqlite..."
	bash $(SCRIPTS_DIR)/build-sqlite.sh build $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

clean-sqlite: clean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-sqlite.sh clean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

distclean-sqlite: distclean-zlib
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-sqlite.sh distclean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

init-sqlite: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-sqlite.sh init $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Zlib
#===================================================================================================

all-zlib: init all-guest-staticlibs
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	echo "Building Zlib..."
	bash $(SCRIPTS_DIR)/build-zlib.sh build $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

clean-zlib:
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-zlib.sh clean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

distclean-zlib:
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-zlib.sh distclean $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

init-zlib: init-repo
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	bash $(SCRIPTS_DIR)/build-zlib.sh init $(ROOT_DIR) $(TOOLCHAIN_DIR) $(SYSROOT_DIR)
endif

#===================================================================================================
# Build Rules for Running and Debugging
#===================================================================================================

# Runs system in release mode.
run: image
ifeq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)
else
	sudo -E $(BINARIES_DIR)/microvm.elf -kernel $(BINARIES_DIR)/kernel.elf -initrd $(IMAGE) 2>&1
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
image: all
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
# Build Rules for Guest Static Libraries
#===================================================================================================

define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1): init
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/lib$(1).a $(LIBRARIES_DIR)/lib$(1).a

check-guest-staticlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
	$(HOST_CARGO_CHECK_CMD) -p $(1) --no-default-features --features=std --all-targets

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/lib$(1).a

clippy-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets

test-guest-staticlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1) --features=staticlib --features=$(LOG_LEVEL)
endef

$(foreach target,$(ALL_GUEST_STATIC_LIBS),$(eval $(call GUEST_STATICLIB_RULES,$(target))))

all-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),all-guest-staticlib-$(target))

check-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),check-guest-staticlib-$(target))

clean-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clean-guest-staticlib-$(target))

clippy-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),clippy-guest-staticlib-$(target))

test-guest-staticlibs: $(foreach target,$(ALL_GUEST_STATIC_LIBS),test-guest-staticlib-$(target))

#===================================================================================================
# Build Rules for Guest Rust Libraries
#===================================================================================================

define GUEST_RLIB_RULES
check-guest-rlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(HOST_CARGO_CHECK_CMD) -p $(1) --no-default-features --features=std --all-targets

clippy-guest-rlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1)
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets

test-guest-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1)
endef

$(foreach target,$(ALL_GUEST_RUST_LIBS),$(eval $(call GUEST_RLIB_RULES,$(target))))

check-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),check-guest-rlib-$(target))

clippy-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),clippy-guest-rlib-$(target))

test-guest-rlibs: $(foreach target,$(ALL_GUEST_RUST_LIBS),test-guest-rlib-$(target))

#===================================================================================================
# Build Rules for Guest Binaries
#===================================================================================================

define GUEST_BINARY_RULES
all-guest-binaries-$(1): init all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-user/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

check-guest-binaries-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=$(LOG_LEVEL)

clean-guest-binaries-$(1): clean-guest-staticlibs
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

clippy-guest-binaries-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=$(LOG_LEVEL)
endef

$(foreach target,$(ALL_GUEST_BINARIES),$(eval $(call GUEST_BINARY_RULES,$(target))))

all-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),all-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks all
	$(MAKE) -C $(SOURCES_DIR)/user all
	$(MAKE) -C $(SOURCES_DIR)/tests all

check-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),check-guest-binaries-$(target))

clean-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clean-guest-binaries-$(target))
	$(MAKE) -C $(SOURCES_DIR)/benchmarks clean
	$(MAKE) -C $(SOURCES_DIR)/user clean
	$(MAKE) -C $(SOURCES_DIR)/tests clean

clippy-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clippy-guest-binaries-$(target))

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

clean-wasmd: clean-wasm-binaries clean-guest-binaries
	$(GUEST_CARGO_CLEAN_CMD) -p wasmd
	$(RM_CMD) $(BINARIES_DIR)/wasmd.elf

clippy-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd

#===================================================================================================
# Build Rules for Kernel Binary
#===================================================================================================

all-kernel: init
	$(KERNEL_CARGO_BUILD_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)-kernel/$(BUILD_MODE)/kernel.elf $(BINARIES_DIR)/kernel.elf

check-kernel:
	$(KERNEL_CARGO_CHECK_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel

clean-kernel:
	$(KERNEL_CARGO_CLEAN_CMD) -p kernel
	$(RM_CMD) $(BINARIES_DIR)/kernel.elf

clippy-kernel:
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

clean-wasm-binaries-$(1):
	$(WASM_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).wasm

clippy-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1)
endef

$(foreach target,$(ALL_WASM_BINARIES),$(eval $(call WASM_BINARY_RULES,$(target))))

all-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),all-wasm-binaries-$(target))

check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),check-wasm-binaries-$(target))

clean-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),clean-wasm-binaries-$(target))

clippy-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),clippy-wasm-binaries-$(target))

#===================================================================================================
# Build Rules for Host Rust Libraries
#===================================================================================================

define HOST_RLIB_RULES
check-host-rlib-$(1):
	$(HOST_CARGO_CHECK_CMD) -p $(1)

clippy-host-rlib-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1)

test-host-rlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1)
endef

$(foreach target,$(ALL_HOST_RUST_LIBS),$(eval $(call HOST_RLIB_RULES,$(target))))

check-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),check-host-rlib-$(target))

clippy-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),clippy-host-rlib-$(target))

test-host-rlibs: $(foreach target,$(ALL_HOST_RUST_LIBS),test-host-rlib-$(target))

#===================================================================================================
# Build Rules for Host Binaries
#===================================================================================================

define HOST_BINARY_RULES
all-host-binaries-$(1): init
	$(HOST_CARGO_BUILD_CMD) -p $(1)
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(1) $(BINARIES_DIR)/$(1).elf

check-host-binaries-$(1):
	$(HOST_CARGO_CHECK_CMD) -p $(1)

clean-host-binaries-$(1):
	$(HOST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).elf

clippy-host-binaries-$(1):
	$(HOST_CARGO_CLIPPY_CMD) -p $(1)
endef

$(foreach target,$(ALL_HOST_BINARIES),$(eval $(call HOST_BINARY_RULES,$(target))))

all-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),all-host-binaries-$(target))

check-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),check-host-binaries-$(target))

clean-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),clean-host-binaries-$(target))

clippy-host-binaries: $(foreach target,$(ALL_HOST_BINARIES),clippy-host-binaries-$(target))

#===================================================================================================
# Build Rules for Microvm Binary
#===================================================================================================

all-microvm: init
	$(HOST_CARGO_BUILD_CMD) $(MICROVM_CARGO_FEATURES) -p microvm
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/microvm $(BINARIES_DIR)/microvm.elf

check-microvm:
	$(HOST_CARGO_CHECK_CMD) $(MICROVM_CARGO_FEATURES) -p microvm

clean-microvm:
	$(HOST_CARGO_CLEAN_CMD) -p microvm
	$(RM_CMD) $(BINARIES_DIR)/microvm.elf

clippy-microvm:
	$(HOST_CARGO_CLIPPY_CMD) $(MICROVM_CARGO_FEATURES) -p microvm

#===================================================================================================
# Rules for Running System Level Tests Using Nanvix Daemon
#===================================================================================================

comma:=,

define TEST_RULE
test-$(1): all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(1)..."
ifeq ($(MACHINE),hyperlight)
	if [ `stat -c%s "bin/$(1).elf"` -gt 16777216 ]; then \
		echo "\033[31mWarning: bin/$(1).elf exceeds 16 MB, skipping test.\033[0m"; \
	else \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/$(1).elf $(2) $(3) $(TIMEOUT); \
	fi
else
	$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/$(1).elf $(2) $(3) $(TIMEOUT)
endif
endif
endef

$(eval $(call TEST_RULE,echo-c,'["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,echo-cpp,'["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,echo-rust-nostd,'["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,hello-c,'[]','Hello$(comma) world from C!'))
$(eval $(call TEST_RULE,hello-cpp,'[]','Hello$(comma) world from C++!'))
$(eval $(call TEST_RULE,linux-app,'[]','ok'))
$(eval $(call TEST_RULE,dlfcn-c,'[]','ok'))
$(eval $(call TEST_RULE,file-c,'[]','ok'))
$(eval $(call TEST_RULE,thread-c,'[]','ok'))
$(eval $(call TEST_RULE,network-c,'[]','ok'))
$(eval $(call TEST_RULE,misc-c,'[]','ok'))
$(eval $(call TEST_RULE,memory-c,'[]','ok'))

define WASM_TEST_RULE
test-$(1): all
ifeq ($(shell basename $(WASM_BINARY)),$(1).wasm)
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(1)..."
ifeq ($(MACHINE),hyperlight)
	if [ `stat -c%s "bin/wasmd.elf"` -gt 16777216 ]; then \
		echo "\033[31mWarning: bin/wasmd.elf exceeds 16 MB, skipping test!\033[0m"; \
	else \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/wasmd.elf $(2) $(3) $(TIMEOUT); \
	fi
else
	$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/wasmd.elf $(2) $(3) $(TIMEOUT)
endif
endif
endif
endef

$(eval $(call WASM_TEST_RULE,echo-wasm-js,'["hello world!"]','hello world!'))
$(eval $(call WASM_TEST_RULE,echo-wasm-rust,'["hello world!"]','hello world!'))
$(eval $(call WASM_TEST_RULE,hello-js,'[]','Hello$(comma) world from JavaScript!'))
$(eval $(call WASM_TEST_RULE,hello-wasm,'[]','Hello$(comma) world!'))

#===================================================================================================
# Rules for Running System Level Tests Using Linux Daemon
#===================================================================================================

define LINUXD_TEST_RULE
test-linuxd-$(2): all
ifneq ($(strip $(filter $(MACHINE),microvm)),)
	@echo "Running Linuxd test $(2)..."
	$(SCRIPTS_DIR)/test-linuxd.sh $(LINUXD_SOCKADDR) $(1)/$(2)$(3) $(4) $(5) $(6) $(TIMEOUT)
endif
endef

$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),echo-c,.elf,'"hello world!"','hello world!'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),echo-cpp,.elf,'"hello world!"','hello world!'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),echo-rust-nostd,.elf,'"hello world!"','hello world!'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),hello-c,.elf,'','','Hello$(comma) world from C!'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),hello-cpp,.elf,'','','Hello$(comma) world from C++!'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),linux-app,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),dlfcn-c,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),file-c,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),network-c,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),misc-c,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(BINARIES_DIR),memory-c,.elf,'','','ok'))
$(eval $(call LINUXD_TEST_RULE,$(SYSROOT_DIR)/bin,python3,,'$(SOURCES_DIR)/user/hello-python/__main__.py','','Hello$(comma) from Python!'))
