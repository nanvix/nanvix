#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#

# Source files.
SRC = $(wildcard *.c)

# Executable files.
EXEC = sh

# Builds init.
all: $(SRC)
	$(CC) $(CFLAGS) -Wl,-T $(LIBDIR)/link.ld $< -o $(BINDIR)/$(EXEC) $(LIBDIR)/$(LIB)

# Cleans compilation files.
clean:
	rm -f $(EXEC) $(BINDIR)/$(EXEC)
