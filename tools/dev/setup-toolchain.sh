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

# macOS support
if [ "x$(uname -rv | grep Darwin)" != "x" ];
then
    if [ "x$(which brew)" == "x" ];
    then
        echo "We only support macOS platforms that use homebrew."
        echo "Please install homebrew (https://brew.sh/) and try again."
        exit 1
    fi

    # Install macOS dependencies
    brew install binutils bochs cdrtools doxygen gcc gnu-sed i386-elf-gcc
    exit 0
fi

# Set working directory.
export CURDIR=`pwd`
export WORKDIR=$CURDIR/nanvix-toolchain
mkdir -p $WORKDIR
cd $WORKDIR

BINUTILS_VERSION=2.30
BINUTILS_PACKAGE=binutils-$BINUTILS_VERSION
BINUTILS_TAR=$BINUTILS_PACKAGE.tar.xz

GCC_VERSION=8.1.0
GCC_PACKAGE=gcc-$GCC_VERSION
GCC_TAR=$GCC_PACKAGE.tar.xz

# Get binutils and GCC.
wget --no-check-certificate "http://ftp.gnu.org/gnu/binutils/$BINUTILS_TAR"
wget --no-check-certificate "http://ftp.gnu.org/gnu/gcc/$GCC_PACKAGE/$GCC_TAR"

# Get required packages.
DIST=$(uname -rv)

case ${DIST,,} in
    *"ubuntu"*|*"debian"*)
        apt-get install -y g++ doxygen genisoimage gdb xz-utils make
        ;;
    *"arch"*)
        pacman -S gcc doxygen cdrtools gdb xz make --needed --noconfirm
        ;;
    *)
        echo "Warning: your distro is not officially supported or tested by us"
esac

# Export variables.
export PREFIX=/usr/local/cross
export TARGET=i386-elf
export CFLAGS=-pipe
export CXXFLAGS=-pipe

# Make TARGET and PATH permanent
sh -c "echo 'export TARGET=$TARGET' > /etc/profile.d/var.sh"
sh -c "echo 'export PATH=\$PATH:$PREFIX/bin' >> /etc/profile.d/var.sh"

# Build binutils.
tar -xvf $BINUTILS_TAR
cd $BINUTILS_PACKAGE
./configure --target=$TARGET --prefix=$PREFIX --disable-nls
make all
make install

# Build GCC.
cd $WORKDIR
tar -xvf $GCC_TAR
cd $GCC_PACKAGE
./contrib/download_prerequisites
./configure --target=$TARGET --prefix=$PREFIX --disable-nls --enable-languages=c --without-headers
make all-gcc
make install-gcc

# Cleans files.
cd $WORKDIR
cd ..
rm -R -f $WORKDIR
