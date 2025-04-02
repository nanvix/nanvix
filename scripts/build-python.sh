#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
TOOLCHAIN_DIR=${2:-$PWD/toolchain}
NANVIX_HOME=${3:-$PWD}

#===================================================================================================
# Global Variables
#===================================================================================================

PYTHON_VERSION=3.12.3
OPT_DIR=$PWD/opt
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

build() {
	# Configure.
	LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld" \
	CFLAGS="-static" \
	CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
	CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
	LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
	LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a -Wl,--end-group" \
	./configure \
		--disable-shared \
		--host=i686-nanvix \
		--build=x86_64-pc-linux-gnu \
		--with-build-python=/usr/bin/python3 \
		--disable-test-modules \
		ac_cv_file__dev_ptmx=no \
		ac_cv_file__dev_ptc=no \
		ac_cv_pthread_is_default=yes

	# Build.
	make all
}

#===================================================================================================

# Check if host system has the required python version
if ! python3 --version | grep -q $PYTHON_VERSION; then
	echo "Python $PYTHON_VERSION is required to build CPython."
	exit 0
fi

# Clone if needed and enter the cpython directory
if [ ! -d $CPYTHON_HOME ];
then
	git clone https://github.com/nanvix/cpython --branch $BRANCH_NAME $CPYTHON_HOME
fi
cd $CPYTHON_HOME

# Save current environment variables.
OLD_LDFLAGS=$LDFLAGS
OLD_CFLAGS=$CFLAGS
OLD_CC=$CC
OLD_CXX=$CXX

# Unset variables that might interfere with the build process.
unset LDFLAGS
unset CFLAGS
unset CC
unset CXX

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
export LDFLAGS=$OLD_LDFLAGS
export CFLAGS=$OLD_CFLAGS
export CC=$OLD_CC
export CXX=$OLD_CXX
