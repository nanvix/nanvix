#!/bin/sh

#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# scripts/setup-bochs.sh - Setup Bochs.
# 
# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

# Set working directory.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR

# Get required packages.
apt-get install libgtk2.0-dev

# Get bochs.
wget "http://sourceforge.net/projects/bochs/files/bochs/2.6.8/bochs-2.6.8.tar.gz"

# Build Bochs
tar -xvf bochs-2.6.8.tar.gz
cd bochs-2.6.8/
./configure --enable-x86-debugger --enable-debugger --enable-debugger-gui
sed -i '/^\<LIBS\>/ s/$/ -lpthread/' Makefile
make all
make install

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
