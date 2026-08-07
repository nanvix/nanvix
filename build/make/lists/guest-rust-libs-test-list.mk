# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Guest Rust library crates that carry host unit tests (a subset of
# ALL_GUEST_RUST_LIBS). One entry per line to minimize merge conflicts.
ALL_GUEST_RUST_LIBS_TEST_LIST := \
	arch \
	bitmap \
	bump-allocator \
	cache \
	cmdline \
	config \
	elf \
	error \
	fat32 \
	hostfs-api \
	type-safe \
	koptions \
	proc \
	raw-array \
	nanvix-slab \
	sorted-vec \
	static_assert \
	libc_assert \
	libc_ctype \
	libc_fnmatch \
	libc_inttypes \
	libc_langinfo \
	libc_libgen \
	libc_locale \
	libc_math \
	libc_mntent \
	libc_regex \
	libc_setjmp \
	libc_signal \
	libc_stdio \
	libc_stdlib \
	libc_string \
	libc_time \
	libc_wchar \
	libc_wctype \
	syslog-macros \
	syslog \
	sys \
	mmio-tag \
	syscall \
	vfs
