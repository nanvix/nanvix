# Copyright(c) 2011-2024 The Maintainers of Nanvix.
# Licensed under the MIT License.

# Toolchain
export CC := $(TOOLCHAIN_DIR)/i486/bin/i486-elf-gcc
export LD := $(TOOLCHAIN_DIR)/i486/bin/i486-elf-ld
export AR := $(TOOLCHAIN_DIR)/i486/bin/i486-elf-ar
export CARGO := $(HOME)/.cargo/bin/cargo
export RUSTC := $(HOME)/.cargo/bin/rustc
export GCC_HOME := $(TOOLCHAIN_DIR)/i486/bin

# Target Configuration
export CFLAGS += -D__x86__

# Compiler Options
export CFLAGS += -nostdlib -ffreestanding -march=pentiumpro -Wstack-usage=4096

# Linker Options
export LDFLAGS += -nostdlib --entry=_do_start
export LDFLAGS += -z common-page-size=0x1000
export LDFLAGS += -z max-page-size=0x1000
