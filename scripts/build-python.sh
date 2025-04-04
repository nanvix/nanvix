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

build() {
	# Configure.
	CC="$TOOLCHAIN_DIR/bin/i686-nanvix-gcc" \
	CXX="$TOOLCHAIN_DIR/bin/i686-nanvix-g++" \
	LD="$TOOLCHAIN_DIR/bin/i686-nanvix-ld" \
	LDFLAGS="-static -T $NANVIX_HOME/build/user/linker/x86/user.ld" \
	CFLAGS="-static -I $SYSROOT_DIR/include" \
	LIBS="-Wl,--start-group $NANVIX_HOME/lib/libposix.a $TOOLCHAIN_DIR/i686-nanvix/lib/libc.a $TOOLCHAIN_DIR/i686-nanvix/lib/libm.a -Wl,--end-group -L $SYSROOT_DIR/lib" \
	./configure \
		--disable-shared \
		--host=i686-nanvix \
		--build=x86_64-pc-linux-gnu \
		--with-build-python=/usr/bin/python3 \
		--disable-test-modules \
		--with-libc=$TOOLCHAIN_DIR/i686-nanvix/lib/libc.a \
		--with-libm=$TOOLCHAIN_DIR/i686-nanvix/lib/libm.a \
		--prefix=$SYSROOT_DIR \
		ac_cv_file__dev_ptmx=no \
		ac_cv_file__dev_ptc=no \
		ac_cv_pthread_is_default=yes \
		ac_cv_pthread=yes \
		ac_cv_kthread=no

	# Build.
	make all

	# Install.
	make install

	# Warn about pyvenv.cfg
	echo "=========================================================================="
	echo " Reminder: Create a 'pyvenv.cfg' file with the following contents:"
	echo ""
	echo " python-home = $NANVIX_HOME"
	echo " include-system-site-packages = false"
	echo " version = $PYTHON_VERSION"
	echo ""
	echo " Place this file in the parent directory of $NANVIX_HOME."
	echo "=========================================================================="
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
export AR=$OLD_AR
export AS=$OLD_AS
export CC=$OLD_CC
export CXX=$OLD_CXX
export CPP=$OLD_CPP
export LD=$OLD_LD
export CFLAGS=$OLD_CFLAGS
export CXXFLAGS=$OLD_CXXFLAGS
export LDFLAGS=$OLD_LD_FLAGS
