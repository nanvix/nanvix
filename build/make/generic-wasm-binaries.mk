# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Per-package rules retained for direct invocation (e.g., make all-wasm-binaries-<pkg>).
define WASM_BINARY_RULES
all-wasm-binaries-$(1): init
	$(WASM_CARGO_BUILD_CMD) -p $(1)
	$(CP_CMD) $(OBJECTS_DIR)/wasm32-wasip1/$(WASM_BUILD_MODE)/$(1).wasm $(BINARIES_DIR)/$(1).wasm

check-wasm-binaries-$(1):
	$(WASM_CARGO_CHECK_CMD) -p $(1)

format-wasm-binaries-$(1):
	$(WASM_CARGO_FMT_CMD) -p $(1)

format-check-wasm-binaries-$(1):
	$(WASM_CARGO_FMT_CMD) -p $(1) --check

clean-wasm-binaries-$(1):
	$(WASM_CARGO_CLEAN_CMD) -p $(1)
	$(RM_CMD) $(BINARIES_DIR)/$(1).wasm

rust-lint-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty --allow-no-vcs

rust-lint-check-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_WASM_BINARIES),$(eval $(call WASM_BINARY_RULES,$(target))))

# Batched targets: single cargo invocations for all WASM binaries.
_WASM_BINS_PKGS := $(foreach pkg,$(ALL_WASM_BINARIES),-p $(pkg))

all-wasm-binaries: init
	$(WASM_CARGO_BUILD_CMD) $(_WASM_BINS_PKGS)
	@for pkg in $(ALL_WASM_BINARIES); do \
		$(CP_CMD) $(OBJECTS_DIR)/wasm32-wasip1/$(WASM_BUILD_MODE)/$$pkg.wasm $(BINARIES_DIR)/$$pkg.wasm; \
	done

check-wasm-binaries:
	$(WASM_CARGO_CHECK_CMD) $(_WASM_BINS_PKGS)

format-wasm-binaries:
	$(WASM_CARGO_FMT_CMD) $(_WASM_BINS_PKGS)

format-check-wasm-binaries:
	$(WASM_CARGO_FMT_CMD) $(_WASM_BINS_PKGS) --check

clean-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),clean-wasm-binaries-$(target))

rust-lint-wasm-binaries:
	$(WASM_CARGO_CLIPPY_CMD) $(_WASM_BINS_PKGS) --fix --allow-dirty --allow-no-vcs

rust-lint-check-wasm-binaries:
	$(WASM_CARGO_CLIPPY_CMD) $(_WASM_BINS_PKGS) -- -D warnings
