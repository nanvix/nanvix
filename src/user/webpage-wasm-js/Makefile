# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

#===============================================================================

export PROGRAM_NAME := webpage-wasm-js

#===============================================================================
# Build Artifacts
#===============================================================================

# Source Files
export SOURCE := index.js

# Binary
export BINARY := $(PROGRAM_NAME).wasm

#===============================================================================
# Build Rules
#===============================================================================

all: $(SOURCE)
	$(JAVY) build $(JAVY_FLAGS) $< -o $(BINARIES_DIR)/$(BINARY) || echo "\033[0;31mSkipping\033[0m $(BINARY)"

clean:
	rm -rf $(BINARIES_DIR)/$(BINARY)
