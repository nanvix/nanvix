#!/bin/bash

cd ..
rm -f Nanvix.img
rm -f initrd.img
cp -f img/blank.img Nanvix.img
cp -f img/initrd.img initrd.img
echo $1 | sudo -S chmod 777 Nanvix.img initrd.img

echo $1 | sudo -S losetup /dev/loop2 initrd.img
echo $1 | sudo -S mount /dev/loop2 /mnt
echo $1 | sudo -S cp bin/init /mnt/etc/
echo $1 | sudo -S cp bin/tsh /mnt/bin/
echo $1 | sudo -S umount /dev/loop2
echo $1 | sudo -S losetup -d /dev/loop2
echo $1 | sudo -S losetup /dev/loop2 Nanvix.img
echo $1 | sudo -S mount /dev/loop2 /mnt
echo $1 | sudo -S cp bin/kernel /mnt/kernel
echo $1 | sudo -S cp initrd.img /mnt/initrd.img
echo $1 | sudo -S cp img/menu.lst /mnt/boot/menu.lst
echo $1 | sudo -S umount /dev/loop2
echo $1 | sudo -S losetup -d /dev/loop2
echo $1 | sudo losetup /dev/loop1 Nanvix.img
cd scripts
echo $1 | sudo bochs -q -f bochsrc.txt
echo $1 | sudo losetup -d /dev/loop1
