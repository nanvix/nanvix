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

function insert {
	losetup /dev/loop2 $1
	mount /dev/loop2 /mnt
}

function eject {
	umount /dev/loop2
	losetup -d /dev/loop2
}

function format {
	losetup /dev/loop2 $1
	mkfs.minix -n 14 -i 4096 -1 /dev/loop2 16318
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
dd if=/dev/zero of=hdd.img bs=16M count=1
format hdd.img
insert hdd.img
cp bin/sbin/* /mnt/sbin/
cp bin/ubin/* /mnt/bin/
eject

# Clone blank images.
cp -f tools/img/blank.img nanvix.img
cp -f tools/img/initrd.img initrd.img

# Build initrd image.
insert initrd.img
cp bin/sbin/* /mnt/sbin/
cp bin/ubin/* /mnt/bin/
eject

# Build nanvix image.
insert nanvix.img
cp bin/kernel /mnt/kernel
cp initrd.img /mnt/initrd.img
cp tools/img/menu.lst /mnt/boot/menu.lst
eject

# House keeping.
rm -f initrd.img

