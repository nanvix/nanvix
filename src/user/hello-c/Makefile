# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := hello-c

#===============================================================================
# Build Artifacts
#===============================================================================

# Libraries
# TODO: Remove grouping to speed up linking.
export LIBC_A := $(TOOLCHAIN_DIR)/i486-nanvix/lib/libc.a
export LIBRARIES := $(LIBC_A)
export LIBRARIES += $(LIBRARIES_DIR)/liblinuxd_lib.a

# Source Files
C_SOURCES := $(wildcard *.c)
ASM_SOURCES := $(BUILD_DIR)/user/crt/$(TARGET)/crt0.S

# Object Files
OBJECTS = $(ASM_SOURCES:.S=.o) $(C_SOURCES:.c=.o)

# Binary
export BINARY := $(PROGRAM_NAME).$(EXEC_FORMAT)

# C Toolchain
export CC := $(TOOLCHAIN_DIR)/i486/bin/i486-elf-gcc

# Compiler Options
export CFLAGS += -nostdlib -ffreestanding -march=pentiumpro -Wstack-usage=4096
export CFLAGS += -I $(TOOLCHAIN_DIR)/i486-nanvix/include

# Linker Options
export LDFLAGS := -z noexecstack -nostdlib -nostartfiles -ffreestanding -T $(BUILD_DIR)/user/linker/$(TARGET)/user.ld

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
	@$GCC) $(CFLAGS) $< -c -o $@
else
	$(CC) $(CFLAGS) $< -c -o $@
endif

# Builds an assembly source file.
%.$(TARGET).o: %.S
ifeq ($(VERBOSE), no)
	@echo [AS] $@
	@$(CC) $(CFLAGS) -z noexecstack $< -c -o $@
else
	$(CC) $(CFLAGS) -z noexecstack $< -c -o $@
endif
