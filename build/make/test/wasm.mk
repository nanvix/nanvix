# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

comma:=,

define WASM_TEST_RULE
test-$(1): all
ifeq ($(shell basename $(WASM_BINARY)),$(1).wasm)
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(1)..."
	$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) bin/wasmd.elf $(2) $(3) $(4) $(TIMEOUT);
endif
endif
endef

$(eval $(call WASM_TEST_RULE,echo-wasm-rust,'','["hello world!"]','hello world!'))
$(eval $(call WASM_TEST_RULE,hello-wasm,'','[]','Hello$(comma) world!'))
