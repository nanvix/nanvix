# 
# Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2016-2018 Davidson Francis <davidsondfgl@gmail.com>
#              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#Building kernel
export CFLAGS += -DBUILDING_KERNEL
export CFLAGS += -DSERIAL_ENABLE -DOR1K_HAVE_SHADOW_GPRS

ifeq ($(TARGET),i386)
	export CFLAGS += -DVGA_ENABLE
	export CFLAGS += -DKEYBOARD_ENABLE
endif

# C source files.
C_SRC = $(wildcard arch/$(TARGET)/*.c) \
        $(wildcard dev/*.c)          \
        $(wildcard dev/8250/*.c)     \
        $(wildcard dev/ata/*.c)      \
        $(wildcard dev/klog/*.c)     \
        $(wildcard dev/ramdisk/*.c)  \
        $(wildcard dev/tty/*.c)      \
        $(wildcard fs/*.c)           \
        $(wildcard fs/minix/*.c)      \
        $(wildcard init/*.c)         \
        $(wildcard lib/*.c)          \
        $(wildcard mm/*.c)           \
        $(wildcard pm/*.c)           \
        $(wildcard sys/*.c)          \
        $(wildcard debug/*.c)        \

# Assembly source files.
ASM_SRC = $(wildcard arch/$(TARGET)/*.S) \

# Object files.
OBJ = $(ASM_SRC:.S=.o) \
	  $(C_SRC:.c=.o)   \

# Executable file.
EXEC = kernel

# Linker configuration options.
export LDFLAGS = -T arch/$(TARGET)/link.ld

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) $(DBGFLAGS) -c -o $@

# Builds object file from assembly source file.
%.o: %.S
	$(CC) $< $(CFLAGS) $(DBGFLAGS) -c $(ASMFLAGS) -o $@

# Builds Nanvix kernel.
all: $(OBJ)
	$(LD) $(LDFLAGS) $(OBJ) -o $(BINDIR)/$(EXEC)

# Cleans object and executable files.
clean:
	@rm -f $(OBJ) 
	@rm -f $(BINDIR)/$(EXEC)
