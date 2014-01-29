#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#
# Builds login.
#

# Source files.
SRC = $(wildcard *.c)

# Executable files.
EXEC = login

# Builds login.
all: $(SRC)
	$(CC) $(CFLAGS) $(LDFLAGS) $(SRC) -o $(BINDIR)/$(EXEC) $(LIBDIR)/$(LIB)

# Cleans compilation files.
clean:
	rm -f $(EXEC) $(BINDIR)/$(EXEC)
