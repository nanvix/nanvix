# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Options
#===============================================================================

# Target Architecture
export TARGET ?= x86

# Target Machine
export MACHINE ?= qemu-pc

# Verbose build?
export VERBOSE ?= no

# Release Version?
export RELEASE ?= no

# Timeout
export TIMEOUT ?= 10

# Log Level
export LOG_LEVEL ?= warn

#===============================================================================
# Directories
#===============================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  ?= $(ROOT_DIR)/bin
export LIBRARIES_DIR ?= $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build
export IMAGE_DIR     ?= $(BUILD_DIR)/iso
export SCRIPTS_DIR   := $(BUILD_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain

#===============================================================================
# Toolchain
#===============================================================================

# Toolchain
export CARGO := $(HOME)/.cargo/bin/cargo
export RUSTC := $(HOME)/.cargo/bin/rustc

# Microvm path
export MICROVM_PATH ?= $(TOOLCHAIN_DIR)/microvm/bin

#===============================================================================
# Build Artifacts
#===============================================================================

# Binary
export EXEC_FORMAT := elf
export NAME := kernel
export BIN := $(NAME).$(EXEC_FORMAT)

# Image
ifeq ($(MACHINE),microvm)
export IMAGE_FORMAT := $(EXEC_FORMAT)
export IMAGE := $(BINARIES_DIR)/$(BIN)
else
export IMAGE_FORMAT := iso
export IMAGE := nanvix.iso
endif

#===============================================================================
# Toolchain Configuration
#===============================================================================

# Cargo
CARGO_FEATURES := --no-default-features
ifneq ($(LOG_LEVEL),)
export CARGO_FEATURES += --features $(LOG_LEVEL)
endif
ifneq ($(MACHINE),)
export CARGO_FEATURES += --features $(MACHINE)
endif

# Cargo Options
ifeq ($(RELEASE), yes)
export CARGO_FLAGS := --release
endif

#===============================================================================
# Build Rules
#===============================================================================

export DEFAULT_GOAL := all

# Builds everything.
# We intentionally cleanup object files not managed by Cargo to ensure correctness.
all: make-dirs
	$(CARGO) build --all $(CARGO_FLAGS) $(CARGO_FEATURES)
ifeq ($(RELEASE), yes)
	cp --preserve target/$(TARGET)/release/$(BIN) $(BINARIES_DIR)/$(BIN)
else
	cp --preserve target/$(TARGET)/debug/$(BIN) $(BINARIES_DIR)/$(BIN)
endif

# Runs clippy.
clippy:
	$(CARGO) clippy $(CARGO_FLAGS) $(CARGO_FEATURES) -- -D warnings

# Cleans up everything.
clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(OBJS)
	rm -rf $(BINARIES_DIR)/$(BIN)

# Creates build directories.
make-dirs:
	@mkdir -p $(BINARIES_DIR)
	@mkdir -p $(LIBRARIES_DIR)

# Builds the system image.
image: all
ifeq ($(IMAGE), $(filter %.iso, $(IMAGE)))
	cp -f $(BINARIES_DIR)/*.$(EXEC_FORMAT) $(IMAGE_DIR)/
	grub-mkrescue  $(IMAGE_DIR) -o $(IMAGE)
endif

# Runs system in release mode.
run: image
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)

# Runs system in debug mode.
debug: image
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --debug $(TIMEOUT)
