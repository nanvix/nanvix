#!/bin/bash

sh ./build-img.sh $1
cd ..
echo $1 | sudo losetup /dev/loop1 Nanvix.img
cd scripts
echo $1 | sudo bochs -q -f bochsrc.txt
echo $1 | sudo losetup -d /dev/loop1
