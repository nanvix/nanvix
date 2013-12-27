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

# Toolchain configuration.
export CFLAGS += -I $(INCDIR)
export CFLAGS += -ansi -pedantic
export CFLAGS += -nostdlib -nostdinc -fno-builtin -fno-stack-protector
export CFLAGS += -Wall -Wextra
export DEBUG = -Werror -g
export RELEASE = -D NDEBUG

# Builds the Nanvix operating system.
all: kernel init

# Builds kernel.
kernel:
	cd src/kernel/ && $(MAKE) all

# Builds init process.
init:
	cd $(SRCDIR)/init && $(MAKE) all

# Cleans compilation files.
clean:
	cd src/kernel/ && $(MAKE) clean
