#
# Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# This file is part of Nanvix.
#
# Nanvix is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Nanvix is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
#

#
# Change this to zero if you wanna a
# non-educational version of the system.
#
export EDUCATIONAL_KERNEL=1

# Target Architecture
export ARCH := i486

# Directories.
export BINDIR   = $(CURDIR)/bin
export SBINDIR  = $(BINDIR)/sbin
export UBINDIR  = $(BINDIR)/ubin
export DOCDIR   = $(CURDIR)/doc
export INCDIR   = $(CURDIR)/include
export LIBDIR   = $(CURDIR)/lib
export DOXYDIR  = $(CURDIR)/doxygen
export SRCDIR   = $(CURDIR)/src
export TOOLSDIR = $(CURDIR)/tools
export TOOLCHAIN_DIR ?= $(CURDIR)/toolchain

# Toolchain
export CC = $(TOOLCHAIN_DIR)/bin/$(ARCH)-elf-gcc
export LD = $(TOOLCHAIN_DIR)/bin/$(ARCH)-elf-ld
export AR = $(TOOLCHAIN_DIR)/bin/$(ARCH)-elf-ar

# Random number for chaos.
export KEY = 13

# Toolchain configuration.
export CFLAGS    = -g -I $(INCDIR)
export CFLAGS   += -DKERNEL_HASH=$(KEY) -DEDUCATIONAL_KERNEL=$(EDUCATIONAL_KERNEL)
export CFLAGS   += -std=c99 -pedantic-errors -fextended-identifiers
export CFLAGS   += -nostdlib -nostdinc -fno-builtin -fno-stack-protector
export CFLAGS   += -Wall -Wextra -Werror
export CFLAGS   += -Wstack-usage=3192 -Wlogical-op
export CFLAGS   += -Wredundant-decls -Wvla
export ASMFLAGS  = -Wa,--divide,--warn
export ARFLAGS   = -vq
export LDFLAGS   = -Wl,-T $(LIBDIR)/link.ld

# Resolves conflicts.
.PHONY: tools

# Builds everything.
all: nanvix documentation

# Builds Nanvix.
nanvix:
	mkdir -p $(BINDIR)
	mkdir -p $(SBINDIR)
	mkdir -p $(UBINDIR)
	cd $(SRCDIR) && $(MAKE) all

# Builds system's image.
image: $(BINDIR)/kernel tools
	mkdir -p $(BINDIR)
	bash $(TOOLSDIR)/build/build-img.sh $(EDUCATIONAL_KERNEL) --build-iso

# Builds documentation.
documentation:
	doxygen $(DOXYDIR)/kernel.config

# Builds tools.
tools:
	mkdir -p $(BINDIR)
	cd $(TOOLSDIR)/build && bash ./build-tools.sh

# Cleans compilation files.
clean:
	@rm -f *.img *.iso
	@rm -rf $(BINDIR) nanvix-iso
	@rm -rf $(DOCDIR)/*-kernel
	cd $(SRCDIR) && $(MAKE) clean
	cd $(TOOLSDIR) && $(MAKE) clean

run-qemu:
	bash $(TOOLSDIR)/run/qemu.sh "x86" "nanvix.iso" "--no-debug"

debug-qemu:
	bash $(TOOLSDIR)/run/qemu.sh "x86" "nanvix.iso" "--debug"
