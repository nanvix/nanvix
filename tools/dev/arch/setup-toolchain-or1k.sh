# 
# Copyright(C)  2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
#               2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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

# Required variables.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/tools/dev/toolchain/or1k
export PREFIX=$WORKDIR
export TARGET=or1k-elf

cd $WORKDIR

# Retrieve the number of processor cores
num_cores=`grep -c ^processor /proc/cpuinfo`

if [ ! "$(ls -A $WORKDIR)" ]; then
	git submodule update --init .
fi

# Build binutils and GDB.
cd binutils*/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls --disable-sim --with-auto-load-safe-path=/ --enable-tui
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
make -j$num_cores all-target-libgcc
make install-gcc
make install-target-libgcc
git checkout .
git clean -f -d

# GCC for Linux
wget "https://github.com/openrisc/musl-cross/releases/download/gcc5.3.0-musl1.1.14/or1k-linux-musl_gcc5.3.0_binutils2.26_musl1.1.14.tgz"
cd $WORKDIR
tar -xvf or1k-linux-musl_gcc5.3.0_binutils2.26_musl1.1.14.tgz

# Back to the current folder
cd $CURDIR
