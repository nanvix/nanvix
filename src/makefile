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

# Conflicts.
.PHONY: kernel
.PHONY: lib
.PHONY: sbin
.PHONY: ubin

# Builds everything.
all: kernel lib sbin ubin
	
# Builds the kernel.
kernel:
	cd kernel/ && $(MAKE) all
	
# Builds the libraries.
lib:
	cd lib/ && $(MAKE) all

# Builds superuser utilities.
sbin: lib
	cd sbin/ && $(MAKE) all

# Builds user utilities.
ubin: lib
	cd ubin/ && $(MAKE) all

# Clean compilation files.
clean:
	cd kernel/ && $(MAKE) clean
	cd lib/ && $(MAKE) clean
	cd sbin/ && $(MAKE) clean
	cd ubin/ && $(MAKE) clean
