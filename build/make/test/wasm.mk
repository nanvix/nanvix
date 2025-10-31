# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

comma:=,

# WebAssembly tests: Only HTTP mode is supported for WASM programs.
# Terminal mode is not available because WASM programs must run through wasmd.
define NANVIXD_HTTP_TEST_RULE
test-nanvixd-http-$(1): all
ifeq ($(shell basename $(WASM_BINARY)),$(1).wasm)
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
	@echo "Running test $(1)..."
	$(SCRIPTS_DIR)/test-nanvixd.sh http $(NANVIXD_SOCKADDR) bin/wasmd.elf $(2) $(3) $(4) $(TIMEOUT);
endif
endif
endef

$(eval $(call NANVIXD_HTTP_TEST_RULE,echo-wasm-rust,'','["hello world!"]','hello world!'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,hello-wasm,'','[]','Hello$(comma) world!'))
