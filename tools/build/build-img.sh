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

#
# Inserts disk in a loop device.
#   $1 Disk image name.
#
function insert {
	losetup /dev/loop2 $1
	mount /dev/loop2 /mnt
}

#
# Ejects current disk from loop device.
#
function eject {
	umount /dev/loop2
	losetup -d /dev/loop2
}

#
# Formats a disk.
#   $1 Disk image name.
#   $2 File system size (in blocks).
#   $3 Number of inodes.
#
function format {
	bin/mkfs.minix $1 $2 $3
	bin/mkdir.minix $1 /sbin
	bin/mkdir.minix $1 /bin
	bin/mkdir.minix $1 /home
	bin/mkdir.minix $1 /dev
	bin/mknod.minix $1 /dev/null 666 c 0 0
	bin/mknod.minix $1 /dev/tty 666 c 0 1
	bin/mknod.minix $1 /dev/ramdisk 666 b 0 0
	bin/mknod.minix $1 /dev/hdd 666 b 0 1
}

#
# Copy files to a disk image.
#   $1 Target disk image.
#
function copy_files {
	
	for file in bin/sbin/*; do
		filename=`basename $file`
		bin/cp.minix $1 $file /sbin/$filename
	done
	
	for file in bin/ubin/*; do
		filename=`basename $file`
		bin/cp.minix $1 $file /bin/$filename
	done
}

# Build HDD image.
dd if=/dev/zero of=hdd.img bs=32M count=1
format hdd.img 4096 16384
copy_files hdd.img

# Build initrd image.
dd if=/dev/zero of=initrd.img bs=512K count=1
format initrd.img 128 512
copy_files initrd.img

# Build nanvix image.
cp -f tools/img/blank.img nanvix.img
insert nanvix.img
cp bin/kernel /mnt/kernel
cp initrd.img /mnt/initrd.img
cp tools/img/menu.lst /mnt/boot/menu.lst
eject
