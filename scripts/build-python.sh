#!/bin/bash

# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Script Arguments
#===================================================================================================

RULE=${1:-build}
NANVIX_HOME=${2:-`git rev-parse --show-toplevel`}
TOOLCHAIN_DIR=${3:-$PWD/toolchain}
SYSROOT_DIR=${4:-$PWD/sysroot}

#===================================================================================================
# Global Variables
#===================================================================================================

OPT_DIR=${NANVIX_HOME}/opt
CROSS_DIR=${SYSROOT_DIR}/cross
CPYTHON_HOME=${OPT_DIR}/cpython

#===================================================================================================
# Clean
#===================================================================================================

make_clean() {
	make clean
}

#===================================================================================================
# Clean Everything
#===================================================================================================

distclean() {
	git clean -fdx
}

#===================================================================================================
# Configure
#===================================================================================================

configure() {
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
		--with-build-python=${CROSS_DIR}/bin/python3 \
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
}

#===================================================================================================
# Configure Cross
#====================================================================================================

configure_cross() {
	LDFLAGS='-m32' \
	CFLAGS="-m32" \
	./configure \
	 	-build=x86_64-pc-linux-gnux32 \
		--host=x86_64-pc-linux-gnux32 \
		--disable-shared \
		--disable-test-modules \
		--prefix=${CROSS_DIR} \
		--exec-prefix=${CROSS_DIR} \
		--with-ensurepip=no \
		--with-pkg-config=no \
		--disable-ipv6 \
		ac_cv_file__dev_ptmx=no \
		ac_cv_file__dev_ptc=no
}

#===================================================================================================
# Make
#===================================================================================================

make_all() {
	make -j `nproc` all
}

#===================================================================================================
# Install
#===================================================================================================

install() {
	make install
}

#===================================================================================================
# Build
#===================================================================================================

build_cross() {
	configure_cross

	make_all

	install

	distclean
}

build() {
	# Check if we need to configure or not.
	if [ ! -f "${CPYTHON_HOME}/Makefile" ]; then
		configure
	fi

	make_all

	install
}

#===================================================================================================
# Init
#===================================================================================================

init() {
	# Nothing to do here.
	return
}

#===================================================================================================

# Fetch submodule if needed.
git submodule update --init $CPYTHON_HOME

# Switch to submodule directory.
cd ${CPYTHON_HOME}

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

# Check if cross-platform toolchain exists and build it if does not.
# TODO: improve detection
if [ ! -f "${CROSS_DIR}/bin/python3" ]; then
	build_cross
fi

case $RULE in
	build)
		build
		;;
	clean)
		make_clean
		;;
	distclean)
		distclean
		;;
	init)
		init
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
