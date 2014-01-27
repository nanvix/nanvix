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
SHELL_CFLAGS = -I $(SHELL_INCDIR) $(CFLAGS)

# Phony list.
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
	cd $(SHELL_SRCDIR) && $(MAKE) all BINDIR="$(SHELL_BINDIR)" DOCDIR="$(SHELL_DOCDIR)" CFLAGS="$(SHELL_CFLAGS)"
	cp $(SHELL_BINDIR)/* $(BINDIR)/$(SHELL_EXEC)
	cp $(SHELL_DOCDIR)/* $(DOCDIR)/man/1/$(SHELL_EXEC).1

# Cleans compilation files.
clean:
	cd kernel/ && $(MAKE) clean
	cd init/ && $(MAKE) clean
	cd tsh/src && $(MAKE) clean
	rm -f $(BINDIR)/$(SHELL_EXEC)
	
