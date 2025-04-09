#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-$PWD}
TOOLCHAIN_DIR=${3:-$PWD/toolchain}
SYSROOT_DIR=${4:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

PYTHON_VERSION=3.12.3
OPT_DIR=$NANVIX_HOME/opt
CPYTHON_HOME=$OPT_DIR/cpython
BRANCH_NAME=nanvix/cpython-$PYTHON_VERSION

#===================================================================================================
# Clean
#===================================================================================================

clean() {
	make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
	git clean -fdx
}

#===================================================================================================
# Build
#===================================================================================================

build_cross() {
	git clean -fdx

	# Configure.
	./configure \
		--disable-shared \
		--disable-test-modules \
		--prefix=$SYSROOT_DIR/cross \
		--exec-prefix=$SYSROOT_DIR/cross \
		--with-ensurepip=no \
		--with-pkg-config=no \
		--disable-ipv6 \
		ac_cv_file__dev_ptmx=no \
		ac_cv_file__dev_ptc=no

	# Build.
	make -j `nproc` all

	# Install.
	make install
}

build() {

	build_cross

	git clean -fdx

	# Configure.
	CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
	CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
	LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
	LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld" \
	CFLAGS="-static" \
	LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a -Wl,--end-group" \
	ZLIB_LIBS="-L $SYSROOT_DIR/lib -lz" \
	ZLIB_CFLAGS="-I $SYSROOT_DIR/include" \
	./configure \
		--disable-shared \
		--build=x86_64-pc-linux-gnux32 \
		--host=i686-nanvix \
		--with-build-python=$SYSROOT_DIR/cross/bin/python3 \
		--disable-test-modules \
		--with-libc=$TOOLCHAIN_DIR/i686-nanvix/lib/libc.a \
		--with-libm=$TOOLCHAIN_DIR/i686-nanvix/lib/libm.a \
		--prefix=$SYSROOT_DIR \
		--exec-prefix=$SYSROOT_DIR \
		--with-ensurepip=no \
		--with-pkg-config=no \
		--disable-ipv6 \
		ac_cv_file__dev_ptmx=no \
		ac_cv_file__dev_ptc=no \
		ac_cv_pthread_is_default=yes \
		ac_cv_pthread=yes \
		ac_cv_kthread=no

	# Build.
	make -j `nproc` all

	# Install.
	make install
}

#===================================================================================================

# Check if host system has the required python version
if ! python3 --version | grep -q $PYTHON_VERSION; then
	echo "Python $PYTHON_VERSION is required to build CPython."
	exit 0
fi

# Fetch submodule if needed and enter the cpython directory.
git submodule update --init opt/cpython && cd $CPYTHON_HOME

# Save current environment variables.
OLD_AR=$AR
OLD_AS=$AS
OLD_CC=$CC
OLD_CXX=$CXX
OLD_CPP=$CPP
OLD_LD=$LD
OLD_CFLAGS=$CFLAGS
OLD_CXXFLAGS=$CXXFLAGS
OLD_LD_FLAGS=$LDFLAGS
OLD_LIBC=$LIBC
OLD_LIBM=$LIBM

# Unset variables that might interfere with the build process.
unset AR
unset AS
unset CC
unset CXX
unset CPP
unset LD
unset CFLAGS
unset CXXFLAGS
unset LDFLAGS
unset LIBC
unset LIBM

case $RULE in
	build)
		build
		;;
	clean)
		clean
		;;
	distclean)
		distclean
		;;
esac

# Restore original environment variables.
export AR=$OLD_AR
export AS=$OLD_AS
export CC=$OLD_CC
export CXX=$OLD_CXX
export CPP=$OLD_CPP
export LD=$OLD_LD
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
export LIBC=$OLD_LIBC
export LIBM=$OLD_LIBM
