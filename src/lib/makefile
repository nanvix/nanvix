# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com> 
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

# Conflicts.
.PHONY: libc

# Builds everything.
all: lib documentation

# Builds libraries
lib: libc

# Builds documentation.
documentation: libc-doc

# Builds C library.
libc:
	cd libc/ && $(MAKE) libc

# Builds C library documentation.
libc-doc:
	cd libc/ && $(MAKE) documentation

# Cleans compilation files.
clean:
	cd libc && $(MAKE) clean
