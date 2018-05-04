# 
# Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com> 
#              2016-2017 Subhra S. Sarkar <rurtle.coder@gmail.com>
#              2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#Newlib defines
export CFLAGS += -DSIGNAL_PROVIDED
export CFLAGS += -DMISSING_SYSCALL_NAMES -D_POSIX_C_SOURCE
export CFLAGS += -U__STRICT_ANSI__

# C source files.
C_SRC = $(wildcard *.c)            \
	$(wildcard arch/$(TARGET)/*.c) \
	$(wildcard assert/*.c)         \
	$(wildcard common/*.c)         \
	$(wildcard ctype/*.c)          \
	$(wildcard dirent/*.c)         \
	$(wildcard errno/*.c)          \
	$(wildcard fcntl/*.c)          \
	$(wildcard locale/*.c)         \
	$(wildcard reent/*.c)          \
	$(wildcard search/*.c)         \
	$(wildcard signal/*.c)         \
	$(wildcard stdio/*.c)          \
	$(wildcard stdlib/*.c)         \
	$(wildcard string/*.c)         \
	$(wildcard stropts/*.c)        \
	$(wildcard sys/acct/*.c)       \
	$(wildcard sys/sem/*.c)        \
	$(wildcard sys/stat/*.c)       \
	$(wildcard sys/time/*.c)       \
	$(wildcard sys/times/*.c)      \
	$(wildcard sys/utsname/*.c)    \
	$(wildcard sys/wait/*.c)       \
	$(wildcard sys/mount/*.c)      \
	$(wildcard termios/*.c)        \
	$(wildcard time/*.c)           \
	$(wildcard unistd/*.c)         \
	$(wildcard utime/*.c)          \
	$(wildcard wchar/*.c)          \

# Assembly source files.
ASM_SRC = $(wildcard arch/$(TARGET)/*.S) \

# Object files.
OBJ = $(ASM_SRC:.S=.o) \
	  $(C_SRC:.c=.o)   \

# Library name.
LIB = libc.a

# Builds the C library.
all: $(OBJ)
	$(AR) $(ARFLAGS) $(LIBDIR)/$(LIB) $^

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) -c -o $@

# Builds object file from assembly source file.
%.o: %.S
	$(CC) $< $(CFLAGS) -c $(ASMFLAGS) -o $@

# Cleans compilation files.
clean:
	@rm -f $(LIBDIR)/$(LIB)
	@rm -f $(OBJ)
