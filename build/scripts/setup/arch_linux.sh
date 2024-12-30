# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Update package repository.
pacman -Syu

# Install base-devel by default.
pacman -S --noconfirm base-devel

# Check if the --extra parameter is supplied.
if [[ "$1" == "--extra" ]]; then
    # Install additional packages.
    pacman -S --noconfirm   \
        bison               \
        clang               \
        doxygen             \
        flex                \
        graphviz            \
        grub                \
        glib2               \
        gmp                 \
        ncurses             \
        pixman              \
        sdl2                \
        mtools              \
        ninja               \
        pkgconf             \
        python-virtualenv   \
        qemu-system-x86     \
        texinfo             \
        xorriso
fi
