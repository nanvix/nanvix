# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Options
#===============================================================================

# Target Architecture
export TARGET ?= x86

# Verbose build?
export VERBOSE ?= no

# Release Version?
export RELEASE ?= no

#===============================================================================
# Directories
#===============================================================================

export ROOT_DIR  := $(CURDIR)
export BUILD_DIR := $(ROOT_DIR)/build

#===============================================================================
# Toolchain
#===============================================================================

# Toolchain
export CARGO ?= $(HOME)/.cargo/bin/cargo

# Cargo Flags
export CARGO_FLAGS += --lib --features=staticlib

#===============================================================================
# Build Artifacts
#===============================================================================

# Binary
export NAME := libposix
export LIB := $(NAME).a

#===============================================================================
# Build Rules
#===============================================================================

# Builds everything.
all: make-dirs
	$(CARGO) build $(CARGO_FLAGS)
ifeq ($(RELEASE), yes)
	cp --preserve target/$(TARGET)/release/$(LIB) $(LIBRARIES_DIR)/$(LIB)
else
	cp --preserve target/$(TARGET)/debug/$(LIB) $(LIBRARIES_DIR)/$(LIB)
endif

# Creates build directories.
make-dirs:
	@mkdir -p $(BINARIES_DIR)

clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(LIBRARIES_DIR)/$(LIBRARY)

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
