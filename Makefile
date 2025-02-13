# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

.DEFAULT_GOAL := all

#===================================================================================================
# Build Options
#===================================================================================================

# Target Architecture
export TARGET ?= x86

# Target Machine
export MACHINE ?= qemu-pc

# Release Version?
export RELEASE ?= no

# Timeout
export TIMEOUT ?= 90

# Enable Microvm profiler?
export PROFILER ?= no

# Target Host CPU
export HOST_CPU ?=

# Log Level
export LOG_LEVEL ?= warn

# Wasm binary to embed in the WASM Daemon
export WASM_BINARY ?= $(BINARIES_DIR)/hello-wasm.wasm
export WASM_BINARY_ARGS ?= ""

# Wasm Daemon Socket Address
export WASMD_SOCKADDR ?= 127.0.0.1:8585

#===================================================================================================
# Directories
#===================================================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  := $(ROOT_DIR)/bin
export LIBRARIES_DIR := $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build
export IMAGE_DIR     := $(BUILD_DIR)/iso
export SCRIPTS_DIR   := $(BUILD_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain
export TARGETS_DIR   := $(BUILD_DIR)/targets
export OBJECTS_DIR   := $(ROOT_DIR)/target

#===================================================================================================
# Libraries and Binaries
#===================================================================================================

# File format for executables.
export EXEC_FORMAT := elf

# File format for system image.
ifeq ($(MACHINE),microvm)
export IMAGE_FORMAT := $(EXEC_FORMAT)
else ifeq ($(MACHINE),hyperlight)
export IMAGE_FORMAT := $(EXEC_FORMAT)
else
export IMAGE_FORMAT := iso
endif

# Image
ifeq ($(IMAGE_FORMAT),iso)
export IMAGE := nanvix.iso
else
export IMAGE := $(BINARIES_DIR)/boottime.$(EXEC_FORMAT)
endif

# Libraries
export LIBC := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
export LIBCXX := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libstdc++.a
export LIBNVX := $(LIBRARIES_DIR)/libnvx.a
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a

#===================================================================================================
# Nanvix Variables
#===================================================================================================

# WASM binary to be embedded in the WASM Daemon
ifneq ($(WASM_BINARY),)
export NANVIX_WASM_BINARY := $(WASM_BINARY)
export NANVIX_WASM_BINARY_BASENAME := $(shell basename $(NANVIX_WASM_BINARY))
export NANVIX_WASM_BINARY_ARGS := $(WASM_BINARY_ARGS)
endif

# Socket address for the WASM Daemon
ifneq ($(WASMD_SOCKADDR),)
export NANVIX_WASMD_SOCKADDR := $(WASMD_SOCKADDR)
endif

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

# C++ Compiler Options
export CXXFLAGS := -std=c++17
export CXXFLAGS += -m32 -march=pentiumpro -Wa,-march=pentiumpro
export CXXFLAGS += -Wall -Wextra -Werror
export CXXFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export CXXFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export CXXFLAGS += -Wvla -Wredundant-decls
export CXXFLAGS += -pedantic-errors
export CXXFLAGS += -Wstack-usage=4096

# Linker Options
export LDFLAGS := -z noexecstack -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

# Optimization Flags
ifeq ($(RELEASE), yes)
export CFLAGS += -O3
export CXXFLAGS += -O3
else
export CFLAGS += -O0
export CFLAGS += -g
export CXXFLAGS += -O0
endif

#===================================================================================================
# Rust Toolchain Configuration
#===================================================================================================

# Tools
export CARGO := $(HOME)/.cargo/bin/cargo
export RUSTC := $(HOME)/.cargo/bin/rustc

# Rust flags for guest target.
export GUEST_RUST_FLAGS :="-C relocation-model=static -C prefer-dynamic=no"
export GUEST_CARGO_FLAGS :=-Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem
export GUEST_CARGO_TARGET := --target $(TARGETS_DIR)/$(TARGET).json
export KERNEL_CARGO_FEATURES := --no-default-features --features $(MACHINE) --features $(LOG_LEVEL)

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
export GUEST_CARGO_BUILD_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) build $(GUEST_CARGO_FLAGS) $(CARGO_PROFILE) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CLEAN_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clean $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)
export GUEST_CARGO_CHECK_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) check $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET) --message-format=json
export GUEST_CARGO_CLIPPY_CMD := RUSTFLAGS=$(GUEST_RUST_FLAGS) $(CARGO) clippy $(GUEST_CARGO_FLAGS) $(GUEST_CARGO_TARGET)

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
ALL_GUEST_RUST_LIBS := bitmap error proc raw-array slab sys

ALL_GUEST_DAEMONS := memd procd
ALL_GUEST_BENCHMARKS := echo boottime matmul
ALL_GUEST_APPLICATIONS := hello-rust
ALL_GUEST_TESTS := testd linux-app
ALL_GUEST_BINARIES := $(ALL_GUEST_DAEMONS) $(ALL_GUEST_BENCHMARKS) $(ALL_GUEST_APPLICATIONS)
ALL_GUEST_BINARIES +=  $(ALL_GUEST_TESTS)

ALL_WASM_BINARIES := hello-wasm

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
	all-microvm

# Performs local initialization.
init:
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
	image-clean

distclean: clean
	$(FORCE_RM_CMD) $(OBJECTS_DIR)
	$(FORCE_RM_CMD) $(LIBRARIES_DIR)
	$(FORCE_RM_CMD) $(BINARIES_DIR)

# Runs clippy.
# TODO: enable clippy for 'guest-staticlibs', 'guest-rlibs' and 'gest-binaries'.
clippy: \
	clippy-kernel \
	clippy-wasm-binaries \
	clippy-host-binaries \
	clippy-microvm

check: \
	check-guest-staticlibs \
	check-guest-rlibs \
	check-guest-binaries \
	check-wasmd \
	check-kernel \
	check-wasm-binaries \
	check-host-binaries \
	check-microvm

run-unit-tests: all \
	test-guest-staticlibs \
	test-guest-rlibs

run-nanvixd-tests: | test-echo test-hello-c test-hello-cpp test-linux-app

#===================================================================================================
# Build Rules for Running and Debugging
#===================================================================================================

# Runs system in release mode.
run: image
ifeq ($(IMAGE_FORMAT),iso)
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)
else
	sudo -E $(BINARIES_DIR)/microvm.elf -kernel $(BINARIES_DIR)/kernel.elf -initrd $(IMAGE) 2>&1
endif

# Runs system in debug mode.
debug: image
ifeq ($(IMAGE_FORMAT),iso)
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --debug $(TIMEOUT)
endif

#===================================================================================================
# Build Rules for System Image
#===================================================================================================

# Builds the system image.
image: all
ifeq ($(IMAGE_FORMAT),iso)
	$(CP_CMD) $(BINARIES_DIR)/*.$(EXEC_FORMAT) $(IMAGE_DIR)/
	$(GRUB_CMD) $(IMAGE_DIR) -o $(IMAGE)
endif

image-clean:
ifeq ($(IMAGE_FORMAT),iso)
	$(RM_CMD) $(IMAGE_DIR)/*.$(EXEC_FORMAT)
	$(RM_CMD) $(IMAGE)
endif

#===================================================================================================
# Build Rules for Guest Static Libraries
#===================================================================================================

define GUEST_STATICLIB_RULES
all-guest-staticlib-$(1):
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=staticlib
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)/$(BUILD_MODE)/lib$(1).a $(LIBRARIES_DIR)/lib$(1).a

check-guest-staticlib-$(1):
	$(GUEST_CARGO_CHECK_CMD) -p $(1)
	$(GUEST_CARGO_CHECK_CMD) -p $(1) --features=staticlib
	$(HOST_CARGO_CHECK_CMD) -p $(1) --no-default-features --features=std --all-targets

clean-guest-staticlib-$(1):
	$(GUEST_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(LIBRARIES_DIR)/lib$(1).a

clippy-guest-staticlib-$(1):
	$(GUEST_CARGO_CLIPPY_CMD) -p $(1) --features=staticlib
	$(HOST_CARGO_CLIPPY_CMD) -p $(1) --no-default-features --features=std --all-targets

test-guest-staticlib-$(1):
	$(HOST_CARGO_TEST_CMD) -p $(1) --features=staticlib
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
all-guest-binaries-$(1): all-guest-staticlibs
	$(GUEST_CARGO_BUILD_CMD) -p $(1) --features=$(LOG_LEVEL)
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)/$(BUILD_MODE)/$(1).elf $(BINARIES_DIR)/$(1).elf

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

clippy-guest-binaries: $(foreach target,$(ALL_GUEST_BINARIES),clippy-guest-binaries-$(target))A

all-wasmd: all-wasm-binaries
	$(GUEST_CARGO_BUILD_CMD) -p wasmd
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)/$(BUILD_MODE)/wasmd.elf $(BINARIES_DIR)/wasmd.elf

check-wasmd:
	$(GUEST_CARGO_CHECK_CMD) -p wasmd

clean-wasmd: clean-wasm-binaries
	$(GUEST_CARGO_CLEAN_CMD) -p wasmd
	$(RM_CMD) $(BINARIES_DIR)/wasmd.elf

clippy-wasmd:
	$(GUEST_CARGO_CLIPPY_CMD) -p wasmd

#===================================================================================================
# Build Rules for Kernel Binary
#===================================================================================================

all-kernel:
	$(GUEST_CARGO_BUILD_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel
	$(CP_CMD) $(OBJECTS_DIR)/$(TARGET)/$(BUILD_MODE)/kernel.elf $(BINARIES_DIR)/kernel.elf

check-kernel:
	$(GUEST_CARGO_CHECK_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel

clean-kernel:
	$(GUEST_CARGO_CLEAN_CMD) -p kernel
	$(RM_CMD) $(BINARIES_DIR)/kernel.elf

clippy-kernel:
	$(GUEST_CARGO_CLIPPY_CMD) $(KERNEL_CARGO_FEATURES) --features $(LOG_LEVEL) -p kernel

#===================================================================================================
# Build Rules for WASM Binaries
#===================================================================================================

define WASM_BINARY_RULES
all-wasm-binaries-$(1):
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
# Build Rules for Host Binaries
#===================================================================================================

define HOST_BINARY_RULES
all-host-binaries-$(1):
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

all-microvm:
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
# Rules for Running System Level tests
#===================================================================================================

test-echo: all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	./scripts/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/echo.elf '["hello world!"]' 'hello world!'
endif

test-hello-c: all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	./scripts/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/hello-c.elf '[]' 'Hello, world from C!'
endif

test-hello-cpp: all
ifneq ($(strip $(filter $(MACHINE),microvm, hyperlight)),)
	./scripts/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/hello-cpp.elf '[]' 'Hello, world from C++!'
endif

test-linux-app: all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	./scripts/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(LINUXD_SOCKADDR) $(SANDBOX_SOCKADDR) bin/linux-app.elf '[]' 'ok'
endif
