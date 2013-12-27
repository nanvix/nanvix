#
# Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
#

# Source files.
SRC = $(wildcard *.c)

# Object files.
OBJ = $(SRC:.c=.release.o)
OBJDBG = $(SRC:.c=.debug.o)

# Executable files.
EXEC = init
EXECDBG = init.debug

# Builds everything.
all: release debug

# Builds release version.
release: $(SRC)
	$(CC) $(CFLAGS) $(RELEASE) -Wl,-T link.ld $< -o $(BINDIR)/$(EXEC)

# Builds debug version.
debug: $(SRC)
	$(CC) $(CFLAGS) $(DEBUG) $< -Wl,-T link.ld -o $(BINDIR)/$(EXECDBG)

# Cleans compilation files.
clean:
	rm -f $(EXEC) $(EXECDBG)
