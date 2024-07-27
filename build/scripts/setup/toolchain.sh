# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

TARGET=$1 # Target
PREFIX=$2 # Prefix

#===============================================================================
# Setup Binutils
#===============================================================================

function setup_binutils
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=2.40

    pushd $PWD

    # Create build directory.
    mkdir -p build-binutils && cd build-binutils

    # Get sources.
    wget https://ftp.gnu.org/gnu/binutils/binutils-$VERSION.tar.xz
    tar -xvf binutils-$VERSION.tar.xz
    cd binutils-$VERSION

    # Build and install.
    ./configure --target=$TARGET --prefix=$PREFIX --disable-nls --disable-sim
    make all
    make install

    popd

    # Cleanup build files.
    rm -rf build-binutils
}

#===============================================================================
# Setup GCC
#===============================================================================

function setup_gcc
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=13.1.0

    pushd $PWD

    # Create build directory.
    mkdir -p build-gcc && cd build-gcc

    # Get sources.
    wget https://ftp.gnu.org/gnu/gcc/gcc-$VERSION/gcc-$VERSION.tar.xz
    tar -xvf gcc-$VERSION.tar.xz
    cd gcc-$VERSION

    ./contrib/download_prerequisites

    # Build and install.
    mkdir build && cd build
    ../configure \
        --target=$TARGET  --prefix=$PREFIX \
        --disable-nls --enable-languages=c --without-headers \
        --disable-multilib $ABI
    make all-gcc all-target-libgcc
    make install-gcc install-target-libgcc

    popd

    # Cleanup build files.
    rm -rf build-gcc
}

#===============================================================================
# Setup GDB
#===============================================================================

function setup_gdb
{
    local TARGET="$1-elf"
    local PREFIX=$2
    local VERSION=13.1

    pushd $PWD

    # Create build directory.
    mkdir -p build-gdb && cd build-gdb

    # Get sources.
	wget https://ftp.gnu.org/gnu/gdb/gdb-$VERSION.tar.xz
	tar -xvf gdb-$VERSION.tar.xz
	cd gdb-$VERSION

	# Build and install.
	./configure \
        --target=$TARGET --prefix=$PREFIX \
        --with-auto-load-safe-path=/ --with-guile=no
	make
	make install

    popd

    # Cleanup build files.
    rm -rf build-gdb
}

#===============================================================================
# Setup Toolchain
#===============================================================================

function setup_toolchain
{
    local TARGET=$1
    local PREFIX=$2/toolchain/$TARGET

	setup_binutils $TARGET $PREFIX
	setup_gcc $TARGET $PREFIX
	setup_gdb $TARGET $PREFIX
}

#===============================================================================
# Main
#===============================================================================

if [ -z "$PREFIX" ];
then
    export PREFIX=$PWD
fi

case "$TARGET" in
	"x86")
        setup_toolchain "i486" $PREFIX
        ;;
    *)
        echo "Unsupported target: $TARGET"
        ;;
esac
