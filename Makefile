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

# FEATURES
export FEATURES ?=

#===============================================================================
# Directories
#===============================================================================

export ROOT_DIR      := $(CURDIR)
export BINARIES_DIR  := $(ROOT_DIR)/bin
export LIBRARIES_DIR := $(ROOT_DIR)/lib
export BUILD_DIR     := $(ROOT_DIR)/build
export IMAGE_DIR     := $(BUILD_DIR)/iso
export SCRIPTS_DIR   := $(BUILD_DIR)/scripts
export SOURCES_DIR   := $(ROOT_DIR)/src
export TOOLCHAIN_DIR ?= $(ROOT_DIR)/toolchain

#===============================================================================
# Libraries and Binaries
#===============================================================================

# Binary
export KERNEL := nanvix.$(EXEC_FORMAT)

# Image
export IMAGE := nanvix.iso

#===============================================================================
# Toolchain
#===============================================================================

include $(BUILD_DIR)/makefile

#===============================================================================
# Build Rules
#===============================================================================

# Builds everything.
all: make-dirs $(OBJS)
	$(MAKE) -C daemons all
	$(MAKE) -C kernel all

# Performs local initialization.
init:
	@git config --local core.hooksPath .githooks

# Creates build directories.
make-dirs: init
	@mkdir -p $(BINARIES_DIR)
	@mkdir -p $(LIBRARIES_DIR)

# Cleans build.
clean:
	$(MAKE) -C daemons clean
	$(MAKE) -C kernel clean
	rm -rf $(LIB)
	rm -rf $(OBJS)
	@rm -f $(IMAGE_DIR)/*.$(EXEC_FORMAT)
	@rm -f $(IMAGE)
	rm -rf $(BINARIES_DIR)/$(BIN)
