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
# Script parameters.
#   $1 Educational kernel?
#   $2 Generate ISO image?
#
EDUCATIONAL_KERNEL=$1
GENERATE_ISO=$2
# Root credentials.
ROOTUID=0
ROOTGID=0

# User credentials.
NOOBUID=1
NOOBUID=1

# macOS compatibility stuff
if [ "x$(uname -rv | grep Darwin)" != "x" ];
then
    # On mac, better use GNU binutils strip rather than
    # Apple preinstalled strip.
    STRIP_TOOL=$(find /usr/local/Cellar/binutils/*/bin -name strip)
    if [ "x$STRIP_TOOL" == "x" ];
    then
        echo "Couldn't find binutils' strip in your homebrew packages."
        echo "Please run 'brew install binutils' and try again."
        exit 1
    fi

    IMG_TOOL=$(which mkisofs)
    if [ "x$IMG_TOOL" == "x" ];
    then
        echo "Could not find mkisofs."
        echo "Please run 'brew install cdrtools' and try again."
        exit 1
    fi

    # On mac, better use GNU sed rather than
    # Apple preinstalled sed.
    SED_TOOL=$(which gsed)
    if [ "x$SED_TOOL" == "x" ];
    then
        echo "Could not find GNU sed."
        echo "Please run 'brew install gnu-sed' and try again."
        exit 1
    fi
else
    # If you didn't have the money to buy a Mac...
    STRIP_TOOL=$(which strip)
    IMG_TOOL=$(which genisoimage)
    SED_TOOL=$(which sed)
fi

#
# Inserts disk in a loop device.
#   $1 Disk image name.
#
function insert {
	losetup $2 $1
	mount $2 /mnt
}

#
# Ejects current disk from loop device.
#
function eject {
	umount $1
	losetup -d $1
}

# Generate passwords file
#   $1 Disk image name.
#
function passwords
{
	file="passwords"

	bin/useradd $file root root $ROOTGID $ROOTUID
	bin/useradd $file noob noob $NOOBUID $NOOBUID

	# Let's care about security...
	if [ "$EDUCATIONAL_KERNEL" == "0" ]; then
		chmod 600 $file
	fi

	bin/cp.minix $1 $file /etc/$file $ROOTUID $ROOTGID

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
	bin/mkfs.minix $1 $2 $3 $ROOTUID $ROOTGID
	bin/mkdir.minix $1 /etc $ROOTUID $ROOTGID
	bin/mkdir.minix $1 /sbin $ROOTUID $ROOTGID
	bin/mkdir.minix $1 /bin $ROOTUID $ROOTGID
	bin/mkdir.minix $1 /home $ROOTUID $ROOTGID
	bin/mkdir.minix $1 /dev $ROOTUID $ROOTGID
	bin/mknod.minix $1 /dev/null 666 c 0 0 $ROOTUID $ROOTGID
	bin/mknod.minix $1 /dev/tty 666 c 0 1 $ROOTUID $ROOTGID
	bin/mknod.minix $1 /dev/klog 666 c 0 2 $ROOTUID $ROOTGID
	bin/mknod.minix $1 /dev/ramdisk 666 b 0 0 $ROOTUID $ROOTGID
	bin/mknod.minix $1 /dev/hdd 666 b 0 1 $ROOTUID $ROOTGID
}

#
# Copy files to a disk image.
#   $1 Target disk image.
#
function copy_files
{
	chmod 666 tools/img/inittab

	# Let's care for security...
	if [ "$EDUCATIONAL_KERNEL" == "0" ]; then
		chmod 600 tools/img/inittab
	fi
	bin/cp.minix $1 tools/img/inittab /etc/inittab $ROOTUID $ROOTGID

	passwords $1

	for file in bin/sbin/*; do
		filename=`basename $file`
		if [[ "$filename" != *.debug ]]; then
			bin/cp.minix $1 $file /sbin/$filename $ROOTUID $ROOTGID
		fi;
	done

	for file in bin/ubin/*; do
		filename=`basename $file`
		if [[ "$filename" != *.debug ]]; then
			bin/cp.minix $1 $file /bin/$filename $ROOTUID $ROOTGID
		fi;
	done
}

#
# Strip a binary from it's debug symbols and
# add a GNU debug link to the original binary
# $1 The binary to strip
#
function strip_binary
{
	if [[ "$1" != *.debug ]]; then
		echo "Stripping $1"

		# Get debug symbols from the file
		# objcopy --compress-debug-sections $1
		cp $1 $1.debug

		# Remove debug symbols from the file
		$STRIP_TOOL --strip-debug --strip-unneeded $1

		objcopy --add-gnu-debuglink=$1.debug $1 2>/dev/null
	fi;

}

strip_binary bin/kernel

for file in bin/sbin/*; do
	strip_binary $file
done

for file in bin/ubin/*; do
	strip_binary $file
done

# Build HDD image.
dd if=/dev/zero of=hdd.img bs=512 count=131072
format hdd.img 32708 16384
copy_files hdd.img

# Build initrd image.
dd if=/dev/zero of=initrd.img bs=512k count=1
format initrd.img 128 512
copy_files initrd.img

# Build nanvix image.
# # Build live nanvix image.
if [ "$GENERATE_ISO" = "--build-iso" ];
then
	mkdir -p nanvix-iso/boot/grub
	cp bin/kernel nanvix-iso/kernel
	cp initrd.img nanvix-iso/initrd.img
	cp tools/img/menu.lst nanvix-iso/boot/grub/menu.lst
	cp tools/img/stage2_eltorito nanvix-iso/boot/grub/stage2_eltorito
    $SED_TOOL -i 's/fd0/cd/g' nanvix-iso/boot/grub/menu.lst
  	$IMG_TOOL -R -b boot/grub/stage2_eltorito -no-emul-boot -boot-load-size 4 \
		-input-charset utf-8 -boot-info-table -o nanvix.iso nanvix-iso
else
	LOOP=$(losetup -f)
	cp -f tools/img/blank.img nanvix.img
	insert nanvix.img $LOOP
	cp bin/kernel /mnt/kernel
	cp initrd.img /mnt/initrd.img
	cp tools/img/menu.lst /mnt/boot/menu.lst
	eject $LOOP
fi
