# 
# Copyright(C)	2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
#		        2016-2017 Subhra Sarkar    <rurtle.coder@gmail.com>
#               2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

# QEMU.
export QEMU_VERSION=2.12.0

# Set working directory.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR

# Retrieve the number of processor cores
num_cores=`grep -c ^processor /proc/cpuinfo`

# Get bochs.
wget "http://wiki.qemu-project.org/download/qemu-$QEMU_VERSION.tar.bz2"

# Get required packages.
apt-get install libglib2.0-dev zlib1g-dev libtool libsdl2-dev libpixman-1-dev dh-autoreconf -y

# Build Bochs
tar -xjvf qemu-$QEMU_VERSION.tar.bz2
cd qemu-$QEMU_VERSION
./configure --target-list=i386-softmmu,or1k-softmmu,or1k-linux-user --enable-sdl
make -j$num_cores all
make install

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
