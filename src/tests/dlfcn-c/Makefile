# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := dlfcn-c

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBRARIES := -Wl,--start-group $(LIBPOSIX) $(LIBC) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.c)

# Object Files
export OBJECTS = $(SOURCES:.c=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS) libs-all
	$(CC) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean: libs-clean
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

libs-all:
	$(CC) libs/sum.c -s -shared -fPIC -o $(LIBRARIES_DIR)/libsum.so -Wl,-soname=$(notdir $(LIBRARIES_DIR))/libsum.so
	$(CC) libs/add.c -s -shared -fPIC -o $(LIBRARIES_DIR)/libadd.so $(LIBRARIES_DIR)/libsum.so

libs-clean:
	rm -rf $(LIBRARIES_DIR)/libsum.so
	rm -rf $(LIBRARIES_DIR)/libadd.so

# Builds C source files.
%.$(TARGET).o: %.c
	$(CC) $(CFLAGS) $< -c -o $@
