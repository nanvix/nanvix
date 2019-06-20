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

# Toolchain configuration.
CFLAGS    = -I $(INCDIR)/fs
CFLAGS   += -std=c99 -pedantic-errors -fextended-identifiers
CFLAGS   += -Wall -Wextra -Werror
CFLAGS   += -D NDEBUG

# Builds everything.
all: cp.minix mkdir.minix mkfs.minix mknod.minix

# Builds cp.minix.
cp.minix: bitmap.c minix.c util.c util.c cp.c
	$(CC) $(CFLAGS) $^ -o $(BINDIR)/$@

# Builds mkdir.minix.
mkdir.minix: bitmap.c minix.c util.c util.c mkdir.c
	$(CC) $(CFLAGS) $^ -o $(BINDIR)/$@

# Builds mkfs.minix.
mkfs.minix: bitmap.c minix.c util.c util.c mkfs.c
	$(CC) $(CFLAGS) $^ -o $(BINDIR)/$@

# Builds mknod.minix.
mknod.minix: bitmap.c minix.c util.c util.c mknod.c
	$(CC) $(CFLAGS) $^ -o $(BINDIR)/$@

# Cleans compilation files.
clean:
	@rm -f $(BINDIR)/*.minix
