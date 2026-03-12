# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := dlfcn-pie-c

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBRARIES := -Wl,--start-group $(LIBPOSIX) $(LIBC) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.c)

# Object Files
export OBJECTS = $(SOURCES:.c=.$(TARGET).$(MACHINE).o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS) libs-all
	$(NANVIX_CC) $(NANVIX_LDFLAGS) -pie -rdynamic -Wl,--no-dynamic-linker $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean: libs-clean
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

libs-all:
	$(NANVIX_CC) libs/mul.c -shared -fPIC -o $(LIBRARIES_DIR)/libmul-pie.so

libs-clean:
	rm -rf $(LIBRARIES_DIR)/libmul-pie.so

# Builds C source files.
%.$(TARGET).$(MACHINE).o: %.c
	$(NANVIX_CC) $(NANVIX_CFLAGS) -fPIE $< -c -o $@
