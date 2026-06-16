# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

SHIM_PACKAGES := \
	nanvix-oci \
	nanvix-shim-core \
	nanvix-shim-proto \
	nanvix-shim-standalone \
	containerd-shim-nanvix-v1

SHIM_CARGO_PACKAGES := $(foreach p,$(SHIM_PACKAGES),-p $(p))

all-nanvix-shim: init
	$(HOST_CARGO_BUILD_CMD) $(SHIM_CARGO_PACKAGES)
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/containerd-shim-nanvix-v1 $(BINARIES_DIR)/containerd-shim-nanvix-v1

check-nanvix-shim:
	@$(HOST_CARGO_CHECK_CMD) $(SHIM_CARGO_PACKAGES)

format-nanvix-shim:
	$(HOST_CARGO_FMT_CMD) -p nanvix-oci
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-core
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-proto
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-standalone
	$(HOST_CARGO_FMT_CMD) -p containerd-shim-nanvix-v1

format-check-nanvix-shim:
	$(HOST_CARGO_FMT_CMD) -p nanvix-oci --check
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-core --check
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-proto --check
	$(HOST_CARGO_FMT_CMD) -p nanvix-shim-standalone --check
	$(HOST_CARGO_FMT_CMD) -p containerd-shim-nanvix-v1 --check

clean-nanvix-shim:
	$(HOST_CARGO_CLEAN_CMD) $(SHIM_CARGO_PACKAGES)
	$(RM_CMD) $(BINARIES_DIR)/containerd-shim-nanvix-v1

rust-lint-nanvix-shim:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(SHIM_CARGO_PACKAGES) --fix --allow-dirty --allow-no-vcs

rust-lint-check-nanvix-shim:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(SHIM_CARGO_PACKAGES) -- -D warnings

test-nanvix-shim:
	$(HOST_CARGO_TEST_CMD) $(SHIM_CARGO_PACKAGES)

test-integration-nanvix-shim:
	$(HOST_CARGO_TEST_CMD) -p containerd-shim-nanvix-v1 --test integration -- --nocapture

install-nanvix-shim:
	./scripts/install-shim.sh
