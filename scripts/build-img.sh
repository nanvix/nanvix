#!/bin/bash

rm -f Nanvix.img
rm -f initrd.img
cp -f img/blank.img Nanvix.img
cp -f img/initrd.img initrd.img
chmod 777 Nanvix.img initrd.img

losetup /dev/loop2 initrd.img
mount /dev/loop2 /mnt
cp bin/init /mnt/etc/
cp bin/login /mnt/etc/
cp bin/* /mnt/bin/
rm /mnt/bin/kernel
rm /mnt/bin/init
rm /mnt/bin/login
umount /dev/loop2
losetup -d /dev/loop2
losetup /dev/loop2 Nanvix.img
mount /dev/loop2 /mnt
cp bin/kernel /mnt/kernel
cp initrd.img /mnt/initrd.img
cp img/menu.lst /mnt/boot/menu.lst
umount /dev/loop2
losetup -d /dev/loop2
