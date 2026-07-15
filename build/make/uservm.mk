# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

USERVM_FEATURES :=
USERVM_FEATURES += $(if $(filter yes,$(PROFILER)),profile-time,)
USERVM_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
USERVM_FEATURES += $(if $(filter yes,$(WHP)),whp,)
USERVM_FEATURES += $(if $(filter yes,$(RELEASE)),nightly-performance-optimizations,)
USERVM_FEATURES := $(strip $(USERVM_FEATURES))
USERVM_CARGO_FEATURES := $(if $(USERVM_FEATURES),--features "$(USERVM_FEATURES)")

all-uservm: init
	$(HOST_CARGO_BUILD_CMD) $(USERVM_CARGO_FEATURES) -p uservm
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/uservm$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/uservm.$(HOST_BIN_EXT)

check-uservm:
	@$(HOST_CARGO_CHECK_CMD) $(USERVM_CARGO_FEATURES) -p uservm

format-uservm:
	$(HOST_CARGO_FMT_CMD) -p uservm

format-check-uservm:
	$(HOST_CARGO_FMT_CMD) -p uservm --check

clean-uservm:
	$(HOST_CARGO_CLEAN_CMD) -p uservm
	$(RM_CMD) $(BINARIES_DIR)/uservm.$(HOST_BIN_EXT)

rust-lint-uservm:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(USERVM_CARGO_FEATURES) -p uservm --fix --allow-dirty --allow-no-vcs

rust-lint-check-uservm:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(USERVM_CARGO_FEATURES) -p uservm -- -D warnings

test-uservm:
	$(HOST_CARGO_TEST_CMD) $(USERVM_CARGO_FEATURES) -p uservm
