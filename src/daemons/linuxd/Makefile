# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Options
#===============================================================================

# Verbose build?
export VERBOSE ?= no

# Release Version?
export RELEASE ?= no

#===============================================================================
# Directories
#===============================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  ?= $(ROOT_DIR)/bin
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
export EXEC_FORMAT := elf
export NAME := linuxd
export BIN := $(NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

# Builds everything.
all: make-dirs
	$(CARGO) build --all $(CARGO_FLAGS)
ifeq ($(RELEASE), yes)
	cp --preserve target/release/$(NAME) $(BINARIES_DIR)/$(BIN)
else
	cp --preserve target/debug/$(NAME) $(BINARIES_DIR)/$(BIN)
endif

# Creates build directories.
make-dirs:
	@mkdir -p $(BINARIES_DIR)

clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(BINARIES_DIR)/$(BIN)

clippy:
	$(CARGO) clippy $(CARGO_FLAGS) -- -D warnings
