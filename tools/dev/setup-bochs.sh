#!/usr/bin/env bash
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

# NOTES:
#   - This script should work in any Debian-based Linux distribution.
#   - You should run this script with superuser privileges.
#

# Set working directory.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR


export CFLAGS="-pipe -O3"
export CXXFLAGS="-pipe -O3"


# Get required packages.
DIST=$(uname -rv)

case ${DIST,,} in
    *"ubuntu"*|*"debian"*)
        apt-get install -y libncurses5-dev
        ;;
    *"arch"*|*"manjaro"*)
        pacman -S ncurses --needed --noconfirm
        ;;
    *)
        echo "Warning: your distro is not officially supported or tested by us"
esac


# Get required packages.

# Get bochs.
wget --no-check-certificate "http://sourceforge.net/projects/bochs/files/bochs/2.6.9/bochs-2.6.9.tar.gz"

# Build Bochs
tar -xvf bochs-2.6.9.tar.gz
cd bochs-2.6.9/
patch < $CURDIR/tools/dev/gdbstub.patch
./configure --with-term --enable-gdb-stub --enable-all-optimizations
sed -i '/^\<LIBS\>/ s/$/ -lpthread/' Makefile
make all
make install

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
