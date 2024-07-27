# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Script Arguments
#===============================================================================

TARGET=$1 # Target
PREFIX=$2 # Prefix

#==============================================================================
# Setup QEMU
#==============================================================================

function setup_qemu
{
    local TARGET="$1-softmmu"
    local PREFIX=$2/toolchain/qemu
    local VERSION=8.1.0

    pushd $PWD

    # Create build directory.
    mkdir -p build-qemu && cd build-qemu

    # Get the sources.
    wget "https://download.qemu.org/qemu-$VERSION.tar.bz2"
    tar -xjvf qemu-$VERSION.tar.bz2
    rm -f qemu-$VERSION.tar.bz2

    # Build and install.
    cd qemu-$VERSION
    ./configure \
        --prefix=$PREFIX --target-list=$TARGET --enable-sdl --enable-curses
    make all
    make install

    popd

    # Cleanup build files.
    rm -rf build-qemu
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
        setup_qemu "i386" $PREFIX
        ;;
    *)
        echo "Unsupported target: $TARGET"
        ;;
esac
