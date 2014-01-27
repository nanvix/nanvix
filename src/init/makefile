#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#

# Source files.
SRC = $(wildcard *.c)

# Executable files.
EXEC = init

# Builds init.
all: $(SRC)
	$(CC) $(CFLAGS) $(LDFLAGS) $(SRC) -o $(BINDIR)/$(EXEC) $(LIBDIR)/$(LIB)

# Cleans compilation files.
clean:
	rm -f $(EXEC) $(BINDIR)/$(EXEC)
