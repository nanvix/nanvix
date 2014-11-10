# 
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com> 
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


# Source files.
SRC = $(wildcard *.c)          \
      $(wildcard ctype/*.c)    \
      $(wildcard dirent/*.c)   \
      $(wildcard errno/*.c)    \
      $(wildcard fcntl/*.c)    \
      $(wildcard signal/*.c)   \
      $(wildcard stdio/*.c)    \
      $(wildcard stdlib/*.c)   \
      $(wildcard string/*.c)   \
      $(wildcard stropts/*.c)  \
      $(wildcard sys/stat/*.c) \
      $(wildcard sys/wait/*.c) \
      $(wildcard termios/*.c)  \
      $(wildcard unistd/*.c)   \

# Source files.
DOC_SRC = $(wildcard *.3)          \
          $(wildcard ctype/*.3)    \
          $(wildcard dirent/*.3)   \
          $(wildcard errno/*.3)    \
          $(wildcard fcntl/*.3)    \
          $(wildcard signal/*.3)   \
          $(wildcard stdio/*.3)    \
          $(wildcard stdlib/*.3)   \
          $(wildcard string/*.3)   \
          $(wildcard stropts/*.3)  \
          $(wildcard sys/stat/*.3) \
          $(wildcard sys/wait/*.3) \
          $(wildcard termios/*.3)  \
          $(wildcard unistd/*.3)   \

# Object files.
OBJ = $(SRC:.c=.o)

# Library name.
LIB = libc.a

# Builds everything.
all: libc documentation

# Builds the C library.
libc: $(OBJ)
	$(AR) $(ARFLAGS) $(LIBDIR)/$(LIB) $^

# Builds C library documentation.
documentation:
	@cp -f $(DOC_SRC) $(DOCDIR)/man/3

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) -c -o $@

# Cleans compilation files.
clean:
	@rm -f $(LIBDIR)/$(LIB)
	@rm -f $(OBJ)
