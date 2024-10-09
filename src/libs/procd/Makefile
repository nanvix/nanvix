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

export CARGO_FLAGS += --features daemon


#===============================================================================
# Build Rules
#===============================================================================

# Builds everything.
all: make-dirs
	$(CARGO) build --all $(CARGO_FLAGS)

# Creates build directories.
make-dirs:
	@mkdir -p $(BINARIES_DIR)

clean:
	$(CARGO) clean
	rm -rf Cargo.lock

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
