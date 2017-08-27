# 
# Copyright(C)	2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#		2016-2017 Subhra Sarkar  <rurtle.coder@gmail.com>
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

# Retrieve the number of processor cores
num_cores=`grep -c ^processor /proc/cpuinfo`

# Get required packages.
apt-get install libgtk2.0-dev

# Get bochs.
wget "http://sourceforge.net/projects/bochs/files/bochs/2.6.8/bochs-2.6.8.tar.gz"

# Build Bochs
tar -xvf bochs-2.6.8.tar.gz
cd bochs-2.6.8/
./configure --enable-x86-debugger --enable-debugger --enable-debugger-gui
sed -i '/^\<LIBS\>/ s/$/ -lpthread/' Makefile
make -j$num_cores all
make install

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
