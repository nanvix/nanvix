# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===================================================================================================
# Build Artifacts
#===================================================================================================

# C source files.
C_SRC=$(wildcard *.c)

# Assembly source files.
ASM_SRC=$(wildcard *.S)

# Object files.
OBJ = $(ASM_SRC:.S=.o) \
	  $(C_SRC:.c=.o)   \

BIN=noop.$(EXE_SUFFIX)

#===================================================================================================
# Toolchain Configuration
#===================================================================================================

# Compiler flags.
CFLAGS := -m32 -nostdlib -ffreestanding -march=pentium -Wall -Wextra -Werror -O3

# Linker flags.
LDFLAGS := -m32 -Wl,--build-id=none -no-pie -nostdlib -nostartfiles -ffreestanding -T $(BUILD_DIR)/link.ld

#===================================================================================================
# Build Targets
#===================================================================================================

# Buidls everything.
all: $(OBJ)
	$(LD) $(LDFLAGS) -o $(BINARIES_DIR)/$(BIN) $^

# Cleans everything.
clean:
	rm -f $(OBJ)
	rm -f $(BINARIES_DIR)/$(BIN)

#===================================================================================================

# Builds a C source file.
%.o: %.c
	$(CC) $(CFLAGS) $< -c -o $@

# Builds an assembly source file.
%.o: %.S
	$(CC) $(CFLAGS) $< -c -o $@
