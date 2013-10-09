#
# Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
#
# makefile - Builds Nanvix.
#

# Directories.
export BINDIR=$(CURDIR)/bin
export INCDIR=$(CURDIR)/include
export LIBDIR=$(CURDIR)/lib

# Toolchain
export CC=$(TARGET)-gcc
export LD=$(TARGET)-ld

# Builds Nanvix.
all: kernel

# Builds kernel.
kernel:
	cd src/kernel/ && $(MAKE) all

# Cleans compilation files.
clean:
	cd src/kernel/ && $(MAKE) clean
