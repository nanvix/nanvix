#
# Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
#
# Builds the Nanvix operating system.

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
export LDFLAGS   = -Wl,-T $(LIBDIR)/link.ld

# Library name.
export LIB = libc.a

# Builds the Nanvix operating system.
all: kernel libc init shell coreutils

# Builds the kernel.
kernel:
	cd $(SRCDIR) && $(MAKE) kernel

# Builds the shell.
shell: libc
	cd $(SRCDIR) && $(MAKE) shell
	
# Builds init.
init: libc
	cd $(SRCDIR) && $(MAKE) init
	
# Builds core utilities.
coreutils: libc
	cd $(SRCDIR) && $(MAKE) coreutils

# Builds the C library.
libc: 
	cd $(LIBDIR) && $(MAKE) all

# Cleans compilation files.
clean:
	cd $(SRCDIR) && $(MAKE) clean
	cd $(LIBDIR) && $(MAKE) clean
