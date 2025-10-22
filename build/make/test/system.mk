# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

comma:=,

# List of supported functions in the L2 VM.
# FIXME (#986): modify tests such that they don't rely on linuxd being invoked
# from the root of the source tree.
MICROVM_L2_BLOCKLIST := dlfcn-c file-c file-rust python3 qjs

define TEST_RULE
test-$(2): all
ifneq ($(strip $(filter $(MACHINE),microvm hyperlight)),)
ifeq ($(L2_VM),yes)
	@if [ ! -f "$(1)/$(2)$(3)" ]; then \
		printf "\033[31mWarning: %s missing, skipping test.\033[0m\n" "$(1)/$(2)$(3)"; \
	elif printf '%s\n' $(MICROVM_L2_BLOCKLIST) | grep -Fxq -- "$(2)"; then \
		printf "\033[31mWarning: Skipping %s on L2 (not supported, see #986).\033[0m\n" "$(2)"; \
	else \
		echo "Running test $(2)..."; \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) "$(1)/$(2)$(3)" $(4) $(5) $(6) $(TIMEOUT); \
	fi
else
	@if [ ! -f "$(1)/$(2)$(3)" ]; then \
		printf "\033[31mWarning: %s missing, skipping test.\033[0m\n" "$(1)/$(2)$(3)"; \
	else \
		echo "Running test $(2)..."; \
		$(SCRIPTS_DIR)/test-nanvixd.sh $(NANVIXD_SOCKADDR) "$(1)/$(2)$(3)" $(4) $(5) $(6) $(TIMEOUT); \
	fi
endif
endif
endef

$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-c,.elf,'','hello world!','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-cpp,.elf,'','["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),echo-rust-nostd,.elf,'','["hello world!"]','hello world!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),hello-c,.elf,'','[]','Hello$(comma) world from C!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),hello-cpp,.elf,'','[]','Hello$(comma) world from C++!'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),linux-app,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),dlfcn-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),file-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),file-rust,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),thread-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),network-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),misc-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),memory-c,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(BINARIES_DIR),arch-rust,.elf,'','[]','ok'))
$(eval $(call TEST_RULE,$(SYSROOT_DIR)/bin,python3,,'$(SOURCES_DIR)/user/hello-python/__main__.py','','Hello$(comma) from Python!'))
$(eval $(call TEST_RULE,$(SYSROOT_DIR)/bin,qjs,,'$(SOURCES_DIR)/user/hello-js/index.js','','Hello$(comma) world from JavaScript!'))
