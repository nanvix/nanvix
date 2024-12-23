# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := hello-c

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
export LIBC := $(TOOLCHAIN_DIR)/i686-nanvix/lib/libc.a
export LIBPOSIX := $(LIBRARIES_DIR)/libposix.a
export LIBRARIES := -Wl,--start-group $(LIBC) $(LIBPOSIX) -Wl,--end-group

# Source Files
export SOURCES := $(wildcard *.c)

# Object Files
export OBJECTS = $(SOURCES:.c=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

# C Toolchain
export CC := $(TOOLCHAIN_DIR)/bin/i686-nanvix-gcc

# Compiler Options
export CFLAGS += -march=pentiumpro -Wstack-usage=4096

# Linker Options
export LDFLAGS := -z noexecstack -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJECTS)
ifeq ($(VERBOSE), no)
	@echo [LD] $(BINARY)
	@$(CC) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)
else
	$(CC) $(LDFLAGS) $(OBJECTS) $(LIBRARIES) -o $(BINARIES_DIR)/$(BINARY)
endif

clean:
	rm -rf $(OBJECTS)
	rm -rf $(BINARIES_DIR)/$(BINARY)

# Builds C source files.
%.$(TARGET).o: %.c
ifeq ($(VERBOSE), no)
	@echo [CC] $@
	@$(CC) $(CFLAGS) $< -c -o $@
else
	$(CC) $(CFLAGS) $< -c -o $@
endif
