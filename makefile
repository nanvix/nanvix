#
# Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
#

# Directories.
export BINDIR = $(CURDIR)/bin
export DOCDIR = $(CURDIR)/doc
export INCDIR = $(CURDIR)/include
export LIBDIR = $(CURDIR)/lib
export SRCDIR = $(CURDIR)/src

# Toolchain
export CC = $(TARGET)-gcc
export LD = $(TARGET)-ld
export AR = $(TARGET)-ar

# Toolchain configuration.
export CFLAGS    = -I $(INCDIR)
export CFLAGS   += -ansi -pedantic
export CFLAGS   += -nostdlib -nostdinc -fno-builtin -fno-stack-protector
export CFLAGS   += -Wall -Wextra -Werror
export CFLAGS   += -D NDEBUG
export ASMFLAGS  = -Wa,--divide,--warn
export ARFLAGS   = -vq

# Library name.
export LIB = libc.a

# Builds the Nanvix operating system.
all: kernel libc init

# Builds the Nanvix kernel.
kernel:
	cd $(SRCDIR)/kernel && $(MAKE) all

# Builds the C library.
libc:
	cd $(LIBDIR) && $(MAKE) all

# Builds init process.
init: libc
	cd $(SRCDIR)/init && $(MAKE) all

# Cleans compilation files.
clean:
	cd src/kernel/ && $(MAKE) clean
	cd $(LIBDIR) && $(MAKE) clean
	cd $(SRCDIR)/init && $(MAKE) clean
