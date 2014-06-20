#
# Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>.
#
# Builds the Nanvix operating system.

# Directories.
export BINDIR    = $(CURDIR)/bin
export DOCDIR    = $(CURDIR)/doc
export INCDIR    = $(CURDIR)/include
export LIBDIR    = $(CURDIR)/lib
export LIBSRCDIR = $(CURDIR)/libsrc
export SRCDIR    = $(CURDIR)/src

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
export LDFLAGS   = -Wl,-T $(LIBSRCDIR)/link.ld

# Library name.
export LIB = libc.a

# Builds the Nanvix operating system.
all: kernel libraries init login shell coreutils

# Builds the kernel.
kernel:
	cd $(SRCDIR) && $(MAKE) kernel

# Builds the shell.
shell: libraries
	cd $(SRCDIR) && $(MAKE) shell
	
# Builds init.
init: libraries
	cd $(SRCDIR) && $(MAKE) init
	
# Builds login.
login: libraries
	cd $(SRCDIR) && $(MAKE) login
	
# Builds core utilities.
coreutils: libraries
	cd $(SRCDIR) && $(MAKE) coreutils

# Builds the C library.
libraries: 
	cd $(LIBSRCDIR) && $(MAKE) all

# Cleans compilation files.
clean:
	cd $(SRCDIR) && $(MAKE) clean
	cd $(LIBSRCDIR) && $(MAKE) clean
