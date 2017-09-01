# 
# Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

# NOTES:
#   - This script should work in any Linux distribution.
#   - You should run this script with superuser privileges.
#

export CURDIR=`pwd`

if [ "$1" = "--dbg" ]; then
	qemu-system-i386 -s -S                                   \
		-drive file=nanvix.iso,format=raw,if=ide,media=cdrom \
		-m 256M                                              \
		-mem-prealloc &
	ddd --debugger "$CURDIR/tools/dev/toolchain/i386/bin/i386-elf-gdb"
elif [ "$1" = "--perf" ]; then
	qemu-system-i386                                         \
		-drive file=nanvix.iso,format=raw,if=ide,media=cdrom \
		-m 256M                                              \
		-mem-prealloc -cpu host --enable-kvm
else
	qemu-system-i386                                         \
		-drive file=nanvix.iso,format=raw,if=ide,media=cdrom \
		-m 256M                                              \
		-mem-prealloc
fi
