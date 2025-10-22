# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

QUICKJS_BINARY := $(SYSROOT_DIR)/bin/qjs

define QUICKJS_TEST_RULE
test-quickjs-$(1): all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
ifeq ($(L2_VM),yes)
	printf "\033[31mWarning: Skipping %s on L2 (not supported, see #986).\033[0m\n" "$(2)";
else
	@if [ -f "$(QUICKJS_BINARY)" ]; then \
		echo "Running test $(1)..."; \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) $(QUICKJS_BINARY) "--std $(SOURCES_DIR)/tests/quickjs/$(1).js" $(2) $(3) $(TIMEOUT); \
	fi
endif
endif
endef

$(eval $(call QUICKJS_TEST_RULE,test_bigint,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_builtin,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_closure,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_cyclic_import,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_language,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_loop,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_std,'','ok'))
$(eval $(call QUICKJS_TEST_RULE,test_worker,'','ok'))

test-quickjs: \
	test-quickjs-test_bigint \
	test-quickjs-test_builtin \
	test-quickjs-test_closure \
	test-quickjs-test_cyclic_import \
	test-quickjs-test_language \
	test-quickjs-test_loop \
	test-quickjs-test_std \
	test-quickjs-test_worker
