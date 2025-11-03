# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

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
	$(WASM_CARGO_CLIPPY_CMD) -p $(1) --fix --allow-dirty

rust-lint-check-wasm-binaries-$(1):
	$(WASM_CARGO_CLIPPY_CMD) -p $(1) -- -D warnings
endef

$(foreach target,$(ALL_WASM_BINARIES),$(eval $(call WASM_BINARY_RULES,$(target))))

all-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),all-wasm-binaries-$(target))

check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),check-wasm-binaries-$(target))

format-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),format-wasm-binaries-$(target))

format-check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),format-check-wasm-binaries-$(target))

clean-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),clean-wasm-binaries-$(target))

rust-lint-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),rust-lint-wasm-binaries-$(target))

rust-lint-check-wasm-binaries: $(foreach target,$(ALL_WASM_BINARIES),rust-lint-check-wasm-binaries-$(target))
