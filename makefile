# 
# Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
#              2016-2017 Davidson Francis <davidsondfgl@gmail.com>
#              2016-2017 Subhra S. Sarkar <rurtle.coder@gmail.com>
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

# Retrieve the number of processor cores
num_cores = `grep -c ^processor /proc/cpuinfo`

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

# Toolchain
export CC = $(TARGET)-gcc
export LD = $(TARGET)-ld
export AR = $(TARGET)-ar

# Random number for chaos.
export KEY = 13

# Toolchain configuration.
export CFLAGS    = -I $(INCDIR)
export CFLAGS   += -DKERNEL_HASH=$(KEY)
export CFLAGS   += -std=c99 -pedantic-errors -fextended-identifiers
export CFLAGS   += -nostdlib -nostdinc -fno-builtin -fno-stack-protector
export CFLAGS   += -Wall -Wextra -Werror
export CFLAGS   += -Wlogical-op
export CFLAGS   += -Wredundant-decls -Wvla
export ASMFLAGS  = -Wa,--divide,--warn
export ARFLAGS   = -vq
export LDFLAGS   = -Wl,-T $(LIBDIR)/link.ld
export DBGFLAGS += -g -fno-omit-frame-pointer

# Resolves conflicts.
.PHONY: tools

# Builds everything.
all: nanvix documentation

# Builds Nanvix.
nanvix:
	mkdir -p $(BINDIR)
	mkdir -p $(SBINDIR)
	mkdir -p $(UBINDIR)
	cd $(SRCDIR) && $(MAKE) -j$(num_cores) all

# Builds Nanvix with debug flags.
nanvix-debug:
	$(MAKE) -j$(num_cores) nanvix

# Builds system's image.
image: $(BINDIR)/kernel tools
	mkdir -p $(BINDIR)
	bash $(TOOLSDIR)/build/build-img.sh --build-iso

# Builds documentation.
documentation:
	doxygen $(DOXYDIR)/kernel.config

# Builds tools.
tools:
	mkdir -p $(BINDIR)
	cd $(TOOLSDIR) && $(MAKE) -j$(num_cores) all

# Cleans compilation files.
clean:
	@rm -f *.img
	@rm -rf $(BINDIR)
	@rm -rf $(DOCDIR)/*-kernel
	cd $(SRCDIR) && $(MAKE) clean
	cd $(TOOLSDIR) && $(MAKE) clean
