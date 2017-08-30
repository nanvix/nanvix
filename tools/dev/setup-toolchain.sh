# 
# Copyright(C)  2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
#               2016-2017 Davidson Francis <davidsondfgl@gmail.com>
#               2016-2017 Subhra Sarkar    <rurtle.coder@gmail.com>
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
export WORKDIR=$CURDIR/tools/dev/toolchain/i386
cd $WORKDIR

# Retrieve the number of processor cores
num_cores=`grep -c ^processor /proc/cpuinfo`

# Get binutils, GDB and GCC.
if [ ! "$(ls -A $WORKDIR)" ]; then
	git submodule update --init .
fi

# Get required packages.
apt-get install g++
apt-get install ddd

# Export variables.
export PREFIX=$WORKDIR/bin
export TARGET=i386-elf
sh -c "echo 'export TARGET=$TARGET' > /etc/profile.d/var.sh"
sh -c "echo 'export PATH=$PATH:$PREFIX/bin' >> /etc/profile.d/var.sh"

# Build binutils.
cd binutils*/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls
make -j$num_cores all
make install
git checkout .
git clean -f -d

# Build GCC.
cd $WORKDIR
cd gcc*/
./contrib/download_prerequisites
mkdir build
cd build
../configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c --without-headers
make -j$num_cores all-gcc
make install-gcc
git checkout .
git clean -f -d

# Build GDB.
cd $WORKDIR
cd gdb*/
./configure --target=$TARGET --prefix=$PREFIX --with-auto-load-safe-path=/
make -j$num_cores
make install
git checkout .
git clean -f -d

# Install genisoimage.
apt-get install genisoimage

# Back to the current folder
cd $CURDIR
