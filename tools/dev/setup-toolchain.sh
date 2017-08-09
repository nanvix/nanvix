#
# Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

# Set working directory.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR

# Get binutils and GCC.
wget "http://ftp.gnu.org/gnu/binutils/binutils-2.25.tar.bz2"
wget "http://ftp.gnu.org/gnu/gcc/gcc-5.3.0/gcc-5.3.0.tar.bz2"

# Get required packages.
apt-get install -y g++ doxygen genisoimage gdb

# Export variables.
export PREFIX=/usr/local/cross
export TARGET=i386-elf
export CFLAGS=-pipe
export CXXFLAGS=-pipe
sh -c "echo 'export TARGET=$TARGET' > /etc/profile.d/var.sh"
sh -c "echo 'export PATH=\$PATH:$PREFIX/bin' >> /etc/profile.d/var.sh"

# Build binutils.
tar -xjvf binutils-2.25.tar.bz2
cd binutils-2.25/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls
make all
make install

# Build GCC.
cd $WORKDIR
tar -xjvf gcc-5.3.0.tar.bz2
cd gcc-5.3.0/
./contrib/download_prerequisites
./configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c --without-headers
make all-gcc
make install-gcc

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
