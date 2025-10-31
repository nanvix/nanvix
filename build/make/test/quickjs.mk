# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

QUICKJS_BINARY := $(SYSROOT_DIR)/bin/qjs

# QuickJS tests: Only HTTP mode is supported for JavaScript programs.
# Terminal mode is not available because QuickJS requires special invocation with script arguments.
define NANVIXD_HTTP_TEST_RULE
test-nanvixd-http-quickjs-$(1): all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
ifeq ($(L2_VM),yes)
	printf "\033[31mWarning: Skipping %s on L2 (not supported, see #986).\033[0m\n" "$(2)";
else
	@if [ -f "$(QUICKJS_BINARY)" ]; then \
		echo "Running test $(1)..."; \
		$(SCRIPTS_DIR)/test-nanvixd.sh http $(NANVIXD_SOCKADDR) $(QUICKJS_BINARY) "--std $(SOURCES_DIR)/tests/quickjs/$(1).js" $(2) $(3) $(TIMEOUT); \
	fi
endif
endif
endef

$(eval $(call NANVIXD_HTTP_TEST_RULE,test_bigint,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_builtin,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_closure,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_cyclic_import,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_language,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_loop,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_std,'','ok'))
$(eval $(call NANVIXD_HTTP_TEST_RULE,test_worker,'','ok'))

test-nanvixd-http-quickjs: \
	test-nanvixd-http-quickjs-test_bigint \
	test-nanvixd-http-quickjs-test_builtin \
	test-nanvixd-http-quickjs-test_closure \
	test-nanvixd-http-quickjs-test_cyclic_import \
	test-nanvixd-http-quickjs-test_language \
	test-nanvixd-http-quickjs-test_loop \
	test-nanvixd-http-quickjs-test_std \
	test-nanvixd-http-quickjs-test_worker
