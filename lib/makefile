#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#

# Source files.
SRC = $(wildcard *.c)          \
      $(wildcard errno/*.c)    \
      $(wildcard fcntl/*.c)    \
      $(wildcard signal/*.c)   \
      $(wildcard stdio/*.c)    \
      $(wildcard stdlib/*.c)   \
      $(wildcard string/*.c)   \
      $(wildcard stropts/*.c)  \
      $(wildcard sys/wait/*.c) \
      $(wildcard termios/*.c)  \
      $(wildcard unistd/*.c)   \

# Object files.
OBJ = $(SRC:.c=.o)

# Builds object file from C source file.
%.o: %.c
	$(CC) $< $(CFLAGS) -c -o $@

# Builds the C library.
all: $(OBJ)
	$(AR) $(ARFLAGS) $(LIBDIR)/$(LIB) $^

# Cleans compilation files.
clean:
	rm -f $(LIBDIR)/$(LIB)
	rm -f $(OBJ)
