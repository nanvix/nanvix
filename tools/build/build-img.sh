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
	losetup /dev/loop2 $1
	mkfs.minix -n 14 -i $3 -1 /dev/loop2 $2
	mount /dev/loop2 /mnt
	mkdir /mnt/sbin/
	mkdir /mnt/bin/
	mkdir /mnt/home
	mkdir /mnt/dev
	mknod -m 666 /mnt/dev/null c 0 0
	mknod -m 666 /mnt/dev/tty c 1 0
	mknod -m 666 /mnt/dev/ramdisk b 0 0
	mknod -m 666 /mnt/dev/hdd b 1 0
	umount /dev/loop2
	losetup -d /dev/loop2
}

# Build HDD image.
dd if=/dev/zero of=hdd.img bs=32M count=1
format hdd.img 16384 4096
insert hdd.img
cp bin/sbin/* /mnt/sbin/
cp bin/ubin/* /mnt/bin/
eject

# Build initrd image.
dd if=/dev/zero of=initrd.img bs=512k count=1
format initrd.img 512 128
insert initrd.img
cp bin/sbin/* /mnt/sbin/
cp bin/ubin/* /mnt/bin/
eject

# Build nanvix image.
cp -f tools/img/blank.img nanvix.img
insert nanvix.img
cp bin/kernel /mnt/kernel
cp initrd.img /mnt/initrd.img
cp tools/img/menu.lst /mnt/boot/menu.lst
eject
