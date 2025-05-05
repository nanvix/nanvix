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
export OBJECTS = $(SOURCES:.c=.$(TARGET).$(MACHINE).o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

# Specify the required relocation types
REQUIRED_TYPES = R_386_RELATIVE R_386_PC32 R_386_GLOB_DAT R_386_32 R_386_JUMP_SLOT

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS) libs-all
	$(CC) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)

clean: libs-clean
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

libs-all:
	$(CC) libs/mul.c -shared -fPIC -o $(LIBRARIES_DIR)/libmul.so
	@for TYPE in $(REQUIRED_TYPES); do \
	if ! readelf -r $(LIBRARIES_DIR)/libmul.so | grep -q $$TYPE; then \
	echo "Error: Relocation type $$TYPE not found in $(LIBRARIES_DIR)/libmul.so"; \
	exit 1; \
	fi \
	done

libs-clean:
	rm -rf $(LIBRARIES_DIR)/libmul.so

# Builds C source files.
%.$(TARGET).$(MACHINE).o: %.c
	$(CC) $(CFLAGS) $< -c -o $@
