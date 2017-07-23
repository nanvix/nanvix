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
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR

# Retrieve the number of processor cores
num_cores=`grep -c ^processor /proc/cpuinfo`

# Get binutils, GDB and GCC.
wget "http://ftp.gnu.org/gnu/binutils/binutils-2.25.tar.bz2"


# Export variables.
export PREFIX=/usr/local/cross
export TARGET=i386-elf
sh -c "echo 'export TARGET=$TARGET' > /etc/profile.d/var.sh"
sh -c "echo 'export PATH=$PATH:$PREFIX/bin' >> /etc/profile.d/var.sh"

# Build binutils.
tar -xjvf binutils-2.25.tar.bz2
cd binutils-2.25/
./configure --target=$TARGET --prefix=$PREFIX --disable-nls
make -j$num_cores all
make install


# Install genisoimage.
apt-get install genisoimage

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
