# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================
# Build Artifacts
#===============================================================================

# Asembly Source Files
ASM := $(BUILD_DIR)/$(TARGET)/crt0.S

# Object Files
OBJS = $(ASM:.S=.$(TARGET).o)

# Objects
export NAME := test
export LIB := lib$(NAME).a
export BIN := $(NAME).$(EXEC_FORMAT)

#===============================================================================
# Toolchain Configuration
#===============================================================================

# Linker Options
export LDFLAGS += -L $(BUILD_DIR)/$(TARGET) -T user.ld
export LDFLAGS += --gc-sections -static --nostdlib --no-whole-archive

#===============================================================================
# Build Rules
#===============================================================================

all: all-lib $(OBJS)
	$(LD) $(LDFLAGS) -o $(BINARIES_DIR)/$(BIN) $(OBJS) $(LIB)

all-lib:
	$(CARGO) build --lib $(CARGO_FLAGS)
ifeq ($(RELEASE), yes)
	@echo [CARGO] $(LIB)
	cp --preserve target/$(TARGET)/release/$(LIB) $(LIB)
else
	cp --preserve target/$(TARGET)/debug/$(LIB) $(LIB)
endif

clean:
	$(CARGO) clean
	rm -rf Cargo.lock
	rm -rf $(LIB)
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
