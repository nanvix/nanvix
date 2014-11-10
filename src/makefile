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
.PHONY: kernel
.PHONY: lib
.PHONY: sbin
.PHONY: ubin

# Builds everything.
all: nanvix documentation

# Builds Nanvix.
nanvix: kernel lib sbin ubin

# Builds documentation.
documentation: kernel-doc lib-doc sbin-doc ubin-doc
	
# Builds the kernel.
kernel:
	cd kernel/ && $(MAKE) kernel

# Builds kernel documentation.
kernel-doc:
	cd kernel/ && $(MAKE) documentation
	
# Builds the libraries.
lib:
	cd lib/ && $(MAKE) lib

# Builds libraries documentation.
lib-doc:
	cd lib/ && $(MAKE) documentation

# Builds superuser utilities.
sbin: lib
	cd sbin/ && $(MAKE) sbin

# Builds superuser utilities documentation.
sbin-doc:
	cd sbin/ && $(MAKE) documentation 

# Builds user utilities.
ubin: lib
	cd ubin/ && $(MAKE) ubin

# Builds user utilities documentation.
ubin-doc:
	cd ubin/ && $(MAKE) documentation
	
# Clean compilation files.
clean:
	cd kernel/ && $(MAKE) clean
	cd lib/ && $(MAKE) clean
	cd sbin/ && $(MAKE) clean
	cd ubin/ && $(MAKE) clean
