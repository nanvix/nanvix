# 
# Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com> 
#
# This file is part of Nanvix.
#
# Nanvix is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# Nanvix is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
#

# Target processor.
PROCESSOR = i386

# Target platform.
ARCH = x86

# C source files.
C_SRC = $(wildcard arch/$(ARCH)/*.c) \
        $(wildcard dev/*.c)          \
        $(wildcard dev/ata/*.c)      \
        $(wildcard dev/klog/*.c)     \
        $(wildcard dev/ramdisk/*.c)  \
        $(wildcard dev/tty/*.c)      \
        $(wildcard fs/*.c)           \
        $(wildcard init/*.c)         \
        $(wildcard lib/*.c)          \
        $(wildcard mm/*.c)           \
        $(wildcard pm/*.c)           \
        $(wildcard sys/*.c)          \

# Assembly source files.
ASM_SRC = $(wildcard arch/$(ARCH)/*.S) \
          $(wildcard core/*.S)         \

# Link script source file.
LD_SRC = link.c

# Object files.
OBJ = $(ASM_SRC:.S=.o) \
	  $(C_SRC:.c=.o)   \

# Executable file.
EXEC = kernel

CFLAGS += -D __NANVIX_KERNEL__

# Linker configuration options.
export LDFLAGS = -T arch/$(ARCH)/link.ld

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) -c -o $@

# Builds object file from assembly source file.
%.o: %.S
	$(CC) $< $(CFLAGS) -c $(ASMFLAGS) -o $@

# Builds Nanvix kernel.
all: $(OBJ)
	$(LD) $(LDFLAGS) $(OBJ) -o $(BINDIR)/$(EXEC)

# Cleans object and executable files.
clean:
	@rm -f $(OBJ) 
	@rm -f $(BINDIR)/$(EXEC)
