#!/usr/bin/env bash

# Copyright(c) 2011-2023 The Maintainers of Nanvix.
# Licensed under the MIT License.

# If you are using Arch Linux distribution use this script.

# Update system and install required packages.
pacman -Syu --noconfirm \
	bison               \
	base-devel          \
	clang               \
	doxygen             \
	flex                \
	gcc                 \
	gdb                 \
	cdrtools            \
	graphviz            \
	grub                \
	ncurses             \
	glibmm-2.68         \
	gmp                 \
	pixman              \
	sdl2                \
	make                \
	mtools              \
	ninja               \
	pkgconf             \
	texinfo             \
	libisoburn          \
	xz                  \
