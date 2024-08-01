# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Artifacts
#===============================================================================

# Asembly Source Files
ASM := $(BUILD_DIR)/$(TARGET)/user/crt0.S

# Object Files
OBJS = $(ASM:.S=.$(TARGET).o)

# Objects
export NAME := testd
export BIN := $(NAME).$(EXEC_FORMAT)

#===============================================================================
# Build Rules
#===============================================================================

all: $(OBJS)
	@PATH=$(GCC_HOME):$(PATH) $(CARGO) build --all $(CARGO_FLAGS)
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
