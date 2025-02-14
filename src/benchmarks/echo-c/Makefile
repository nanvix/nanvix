# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := echo-c

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBRARIES := -Wl,--start-group $(LIBC) $(LIBPOSIX) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.c)

# Object Files
export OBJECTS = $(SOURCES:.c=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS)
	$(CC) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean:
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

# Builds C source files.
%.$(TARGET).o: %.c
	$(CC) $(CFLAGS) $< -c -o $@
