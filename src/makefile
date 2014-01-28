#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# Builds the kernel, system utilities and user applications.
#

# Shell variables.
SHELL_EXEC = tsh
SHELL_BINDIR = $(CURDIR)/tsh/bin
SHELL_DOCDIR = $(CURDIR)/tsh/doc
SHELL_INCDIR = $(CURDIR)/tsh/include
SHELL_SRCDIR = $(CURDIR)/tsh/src
SHELL_LIBS   = $(LIBDIR)/$(LIB)
SHELL_CFLAGS = -I $(SHELL_INCDIR) $(CFLAGS)

# Core utilities variables.
COREUTILS_BINDIR = $(CURDIR)/coreutils/bin
COREUTILS_DOCDIR = $(CURDIR)/coreutils/doc
COREUTILS_SRCDIR = $(CURDIR)/coreutils/src
COREUTILS_LIBS   = $(LIBDIR)/$(LIB)
COREUTILS_CFLAGS = $(CFLAGS)

# Phony list.
.PHONY: coreutils
.PHONY: init
.PHONY: kernel
.PHONY: shell

# Builds init.
init:
	cd init/ && $(MAKE) all
	
# Builds the kernel.
kernel:
	cd kernel/ && $(MAKE) all

# Builds the shell.
shell:
	cd $(SHELL_SRCDIR) && $(MAKE) all BINDIR="$(SHELL_BINDIR)" DOCDIR="$(SHELL_DOCDIR)" CFLAGS="$(SHELL_CFLAGS)" LIBS="$(SHELL_LIBS)"
	cp $(SHELL_BINDIR)/* $(BINDIR)/$(SHELL_EXEC)
	cp $(SHELL_DOCDIR)/* $(DOCDIR)/man/1/$(SHELL_EXEC).1

# Builds core utilities.
coreutils:
	cd $(COREUTILS_SRCDIR) && $(MAKE) all BINDIR="$(COREUTILS_BINDIR)" DOCDIR="$(COREUTILS_DOCDIR)" CFLAGS="$(COREUTILS_CFLAGS)" LIBS="$(COREUTILS_LIBS)"
	cp $(COREUTILS_BINDIR)/* $(BINDIR)/
	cp $(COREUTILS_DOCDIR)/* $(DOCDIR)/man/1/

# Cleans compilation files.
clean:
	cd kernel/ && $(MAKE) clean
	cd init/ && $(MAKE) clean
	cd tsh/src && $(MAKE) clean
	rm -f $(BINDIR)/*
