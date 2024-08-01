# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

DEFAULT_GOAL := all

#===============================================================================
# Toolchain Configuration
#===============================================================================

include $(BUILD_DIR)/$(TARGET)/makefile

# Compiler Options
export CFLAGS += -std=c17
export CFLAGS += -Wall -Wextra -Werror
export CFLAGS += -Winit-self -Wswitch-default -Wfloat-equal -Wno-pointer-arith
export CFLAGS += -Wundef -Wshadow -Wuninitialized -Wlogical-op
export CFLAGS += -Wvla -Wredundant-decls
export CFLAGS += -pedantic-errors
export CFLAGS += -Wstack-usage=4096

# Optimization Flags
ifeq ($(RELEASE), yes)
export CFLAGS += -O3
else
export CFLAGS += -O0
export CFLAGS += -g
endif

# Cargo Options
ifeq ($(RELEASE), yes)
export CARGO_FLAGS += --release
endif

# Archiver Options
export ARFLAGS = rc

# File format for executables.
export EXEC_FORMAT := elf

#===============================================================================
# Run and Debug Rules
#===============================================================================

# Builds the system image.
image: all
	@cp -f $(BINARIES_DIR)/*.$(EXEC_FORMAT) $(IMAGE_DIR)/
	@grub-mkrescue  $(IMAGE_DIR) -o $(IMAGE)

# Runs system in release mode.
run: image
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --no-debug $(TIMEOUT)

# Runs system in debug mode.
debug: image
	bash $(SCRIPTS_DIR)/run.sh $(TARGET) $(MACHINE) $(IMAGE) --debug $(TIMEOUT)
