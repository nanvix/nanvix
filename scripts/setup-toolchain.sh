# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# scripts/setup-toolchain.sh - Setup development toolchain.
# 
# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

# Set working directory.
mkdir ~/nanvix-toolchain
export WORKDIR=~/nanvix-toolchain
cd $WORKDIR

# Get binutils and GCC.
wget "http://ftp.gnu.org/gnu/binutils/binutils-2.24.tar.bz2"
wget "http://ftp.gnu.org/gnu/gcc/gcc-4.9.0/gcc-4.9.0.tar.bz2"

# Get required packages.
apt-get install build-essential libmpc-dev

# Export variables.
export PREFIX=/usr/local/cross
export TARGET=i386-elf
sh -c "echo 'export TARGET=$TARGET' > /etc/profile.d/var.sh"
sh -c "echo 'export PATH=$PATH:$PREFIX/bin' >> /etc/profile.d/var.sh"

# Build binutils.
tar -xjvf binutils-2.24.tar.bz2
cd binutils-2.24/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls
make all
make install

# Build GCC.
cd $WORKDIR
tar -xjvf gcc-4.9.0.tar.bz2
cd gcc-4.9.0/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c --without-headers
make all-gcc
make install-gcc

# Cleans files.
cd $WORKDIR
rm -R -f $WORKDIR/*
