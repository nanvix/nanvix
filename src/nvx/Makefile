# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Options
#===============================================================================

# Target Architecture
export TARGET := x86

# Verbose build?
export VERBOSE ?= no

# Release Version?
export RELEASE ?= no

#===============================================================================
# Directories
#===============================================================================

export ROOT_DIR      := $(CURDIR)
export LIBRARIES_DIR ?= $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build

#===============================================================================
# Toolchain
#===============================================================================

# Toolchain
export CARGO ?= $(HOME)/.cargo/bin/cargo

#===============================================================================
# Build Artifacts
#===============================================================================

# Binary
export NAME := libnvx
export LIBRARY := $(NAME).a

#===============================================================================
# Build Rules
#===============================================================================

# Builds everything.
all: make-dirs
	$(CARGO) build --all $(CARGO_FLAGS)
ifeq ($(RELEASE), yes)
	cp --preserve target/$(TARGET)/release/$(LIBRARY) $(LIBRARIES_DIR)/$(LIBRARY)
else
	cp --preserve target/$(TARGET)/debug/$(LIBRARY) $(LIBRARIES_DIR)/$(LIBRARY)
endif

# Creates build directories.
make-dirs:
	@mkdir -p $(LIBRARIES_DIR)

clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(LIBRARIES_DIR)/$(LIBRARY)

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
