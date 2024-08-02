# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Artifacts
#===============================================================================

# Asembly Source Files
ASM = $(wildcard src/hal/arch/x86/*.S)

# Object Files
OBJS = $(ASM:.S=.$(TARGET).o)

# Objects
export NAME := kernel
export BIN := $(NAME).$(EXEC_FORMAT)

#===============================================================================
# Toolchain Configuration
#===============================================================================

# Cargo
ifneq ($(FEATURES),)
	export CARGO_FEATURES := --features $(FEATURES)
endif

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJS)
	@PATH=$(GCC_HOME):"$(PATH)" $(CARGO) build --all $(CARGO_FLAGS) $(CARGO_FEATURES)
ifeq ($(RELEASE), yes)
	cp --preserve target/$(TARGET)/release/$(BIN) $(BINARIES_DIR)/$(BIN)
else
	cp --preserve target/$(TARGET)/debug/$(BIN) $(BINARIES_DIR)/$(BIN)
endif

clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(OBJS)
	rm -rf $(BINARIES_DIR)/$(BIN)

# Builds an assembly source file.
%.$(TARGET).o: %.S
ifeq ($(VERBOSE), no)
	@echo [CC] $@
	@$(CC) $(CFLAGS) $< -c -o $@
else
	$(CC) $(CFLAGS) $< -c -o $@
endif
