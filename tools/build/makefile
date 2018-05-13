# 
# Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

# Toolchain
ifeq ($(TARGET),i386)
	CC = gcc
else
ifeq ($(TARGET),or1k)
	CC = $(TOOLSDIR)/dev/toolchain/$(TARGET)/or1k-linux-musl/bin/or1k-linux-musl-gcc
endif
endif

# Toolchain configuration.
CFLAGS    = -I $(INCDIR)/nanvix -DKERNEL_HASH=$(KEY)
CFLAGS   += -std=c99 -pedantic-errors -fextended-identifiers
CFLAGS   += -Wall -Wextra -Werror
CFLAGS   += -D NDEBUG
CFLAGS   += --static

# Builds everything.
all: useradd

# Builds cp.minix.
useradd: useradd.c
	$(CC) $(CFLAGS) $^ -o $(BINDIR)/$@

# Cleans compilation files.
clean:
	@rm -f $(BINDIR)/useradd
