# 
# Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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

# Root credentials.
ROOTUID=0
ROOTGID=0

# User credentials.
NOOBUID=1
NOOBUID=1

LOOP=$(losetup -f)

# Toolchain
STRIP=$TOOLSDIR/dev/toolchain/$TARGET/bin/$TARGET-elf-strip
OBJCOPY=$TOOLSDIR/dev/toolchain/$TARGET/bin/$TARGET-elf-objcopy

# Checks if TARGET is OR1K, if so, use Qemu
if [ "$TARGET" = "or1k" ];
then
	QEMU_VIRT=qemu-or1k
fi

#
# Inserts disk in a loop device.
#   $1 Disk image name.
#
function insert {
	losetup $LOOP $1
	mount $LOOP /mnt
}

#
# Ejects current disk from loop device.
#
function eject {
	umount $LOOP
	losetup -d $LOOP
}

# Generate passwords file
#   $1 Disk image name.
#
function passwords
{
	file="passwords"
	
	$QEMU_VIRT bin/useradd $file root root $ROOTGID $ROOTUID
	$QEMU_VIRT bin/useradd $file noob noob $NOOBUID $NOOBUID

	chmod 600 $file
	
	$QEMU_VIRT bin/cp.minix $1 $file /etc/$file $ROOTUID $ROOTGID
	
	# House keeping.
	rm -f $file
}

#
# Formats a disk.
#   $1 Disk image name.
#   $2 File system size (in blocks).
#   $3 Number of inodes.
#
function format {
	$QEMU_VIRT bin/mkfs.minix $1 $2 $3 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /etc $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /sbin $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /bin $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /home $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /home/rep1 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /home/rep2 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /dev $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mkdir.minix $1 /home/mysem/ $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/null 666 c 0 0 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/tty 666 c 0 1 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/klog 666 c 0 2 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/ramdisk 666 b 0 0 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/ramdisk1 666 b 1 0 $ROOTUID $ROOTGID
	$QEMU_VIRT bin/mknod.minix $1 /dev/hdd 666 b 0 1 $ROOTUID $ROOTGID
}

#
# Copy files to a disk image.
#   $1 Target disk image.
#
function copy_files
{
	chmod 666 tools/img/inittab
	chmod 600 tools/img/inittab
	
	$QEMU_VIRT bin/cp.minix $1 tools/img/inittab /etc/inittab $ROOTUID $ROOTGID
	
	passwords $1
	
	for file in bin/sbin/*; do
		filename=`basename $file`
		$QEMU_VIRT bin/cp.minix $1 $file /sbin/$filename $ROOTUID $ROOTGID
	done
	
	for file in bin/ubin/*; do
		filename=`basename $file`
		$QEMU_VIRT bin/cp.minix $1 $file /bin/$filename $ROOTUID $ROOTGID
	done
}

# Get debug symbols from kernel
$OBJCOPY --only-keep-debug bin/kernel bin/kernel.sym

# Remove debug symbols from kernel
$STRIP --strip-debug bin/kernel

# Build HDD image.
dd if=/dev/zero of=hdd.img bs=1024 count=65536
format hdd.img 1024 32768
copy_files hdd.img

# Build initrd image.
dd if=/dev/zero of=initrd.img bs=1024 count=1152
format initrd.img 1152 512
copy_files initrd.img
initrdsize=`stat -c %s initrd.img`
maxsize=`grep "INITRD_SIZE" include/nanvix/config.h | cut -d" " -f 13`
maxsize=`printf "%d\n" $maxsize`
if [ $initrdsize -gt $maxsize ]; then
	echo "NOT ENOUGH SPACE ON INITRD"
	echo "INITRD SIZE is $initrdsize"
	rm *.img
	exit -1
fi 

# Build live nanvix image.
if [ "$1" = "--build-iso" ];
then
	mkdir -p nanvix-iso/boot/grub
	cp bin/kernel nanvix-iso/kernel
	cp initrd.img nanvix-iso/initrd.img
	cp tools/img/menu.lst nanvix-iso/boot/grub/menu.lst
	cp tools/img/stage2_eltorito nanvix-iso/boot/grub/stage2_eltorito
	sed -i 's/fd0/cd/g' nanvix-iso/boot/grub/menu.lst
	genisoimage -R -b boot/grub/stage2_eltorito -no-emul-boot -boot-load-size 4 \
		-input-charset utf-8 -boot-info-table -o nanvix.iso nanvix-iso
else
	cp -f tools/img/blank.img nanvix.img
	insert nanvix.img
	cp bin/kernel /mnt/kernel
	cp initrd.img /mnt/initrd.img
	cp tools/img/menu.lst /mnt/boot/menu.lst
	eject
fi

