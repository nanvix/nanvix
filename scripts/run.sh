# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# scripts/run.sh - Runs Nanvix in Bochs.
# 
# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

losetup /dev/loop1 Nanvix.img
bochs -q -f scripts/bochsrc.txt
losetup -d /dev/loop1
