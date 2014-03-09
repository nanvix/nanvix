#!/bin/bash

losetup /dev/loop1 Nanvix.img
bochs -q -f scripts/bochsrc.txt
losetup -d /dev/loop1
