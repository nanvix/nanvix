# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Guest Rust library crates (rlibs), one entry per line so that adding a crate
# touches a single line and rarely conflicts across branches. Consumed by
# build/make/generic-guest-rlibs.mk.
ALL_GUEST_RUST_LIBS := \
	arch \
	bitmap \
	bump-allocator \
	cache \
	cmdline \
	config \
	elf \
	error \
	fat32 \
	type-safe \
	koptions \
	nvx \
	proc \
	raw-array \
	nanvix-slab \
	sorted-vec \
	static_assert \
	sysapi \
	syscall \
	sysalloc \
	syslog-macros \
	syslog \
	sys \
	libc_arpa_inet \
	libc_assert \
	libc_ctype \
	libc_dirent \
	libc_dlfcn \
	libc_errno \
	libc_fnmatch \
	libc_ftw \
	libc_glob \
	libc_grp \
	libc_inttypes \
	libc_langinfo \
	libc_libgen \
	libc_locale \
	libc_math \
	libc_mntent \
	libc_netdb \
	libc_poll \
	libc_pthread \
	libc_pwd \
	libc_regex \
	libc_setjmp \
	libc_signal \
	libc_stdio \
	libc_stdlib \
	libc_string \
	libc_sys_ioctl \
	libc_sys_resource \
	libc_sys_stat \
	libc_sys_statvfs \
	libc_sys_time \
	libc_sys_times \
	libc_sys_un \
	libc_sys_uio \
	libc_sys_utsname \
	libc_termios \
	libc_time \
	libc_unistd \
	libc_utime \
	libc_wchar \
	libc_wctype \
	mmio-tag \
	multiimage \
	vfs-bench-common
