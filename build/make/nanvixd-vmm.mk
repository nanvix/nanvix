# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Build rules for the `nanvixd-vmm` crate: the Nanvix standalone daemon running
# on top of the OpenVMM virtualization stack.
#
# This is an OPT-IN target: it is deliberately NOT part of `all-nanvix` unless
# WITH_OPENVMM=yes, because it pulls a large set of OpenVMM crates from GitHub
# (network access required on the first build) and adds significant compile
# time. Build it explicitly with either of:
#
#   ./z build -- all-nanvixd-vmm        # build just this crate
#   ./z build -- WITH_OPENVMM=yes       # include it in the full `all` build
#
# The crate is decoupled from the sibling OpenVMM checkout for its *sources*:
# all OpenVMM libraries are git dependencies pinned to a specific revision (see
# src/utils/nanvixd-vmm/Cargo.toml), and the `[patch.crates-io]` entries it
# needs are mirrored in the workspace root Cargo.toml. MEMORY_SIZE_BYTES is
# exported by this Makefile, so no external wrapper script is required.
#
# One external build tool is still required: `protoc` (Protocol Buffers
# compiler), used by transitive OpenVMM build dependencies (e.g. `tdisp_proto`
# via `prost-build`). It is resolved at build time in this order:
#   1. an explicit PROTOC environment variable,
#   2. a `protoc` on PATH (e.g. `apt-get install protobuf-compiler`),
#   3. the copy restored under the sibling OpenVMM checkout, if present.

# The crate ships two binaries.
NANVIXD_VMM_BIN := nanvixd-vmm
NANVIXD_VMM_WARMSTART_BIN := nanvixd-vmm-warmstart

# Fallback protoc location: the copy OpenVMM restores under its `.packages/`.
NANVIXD_VMM_OPENVMM_PROTOC := $(ROOT_DIR)/../OpenVMM/.packages/Google.Protobuf.Tools/tools/protoc

# Shell snippet that resolves `protoc` into $$PROTOC_BIN, or fails with a helpful
# message. Use it at the start of any recipe that compiles the crate, e.g.:
#   @$(call nanvixd_vmm_resolve_protoc); \
#   PROTOC="$$PROTOC_BIN" $(HOST_CARGO_BUILD_CMD) -p nanvixd-vmm
define nanvixd_vmm_resolve_protoc
PROTOC_BIN="$${PROTOC:-}"; \
if [ -z "$$PROTOC_BIN" ]; then PROTOC_BIN="$$(command -v protoc || true)"; fi; \
if [ -z "$$PROTOC_BIN" ] && [ -x "$(NANVIXD_VMM_OPENVMM_PROTOC)" ]; then \
	PROTOC_BIN="$(NANVIXD_VMM_OPENVMM_PROTOC)"; \
fi; \
if [ -z "$$PROTOC_BIN" ]; then \
	echo "ERROR: protoc not found. Install it (e.g. 'apt-get install protobuf-compiler')," >&2; \
	echo "       set the PROTOC environment variable, or restore OpenVMM's packages." >&2; \
	exit 1; \
fi; \
echo "[nanvixd-vmm] PROTOC=$$PROTOC_BIN"
endef

all-nanvixd-vmm: init
	@$(nanvixd_vmm_resolve_protoc); \
	PROTOC="$$PROTOC_BIN" $(HOST_CARGO_BUILD_CMD) -p nanvixd-vmm
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(NANVIXD_VMM_BIN)$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/$(NANVIXD_VMM_BIN).$(HOST_BIN_EXT)
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/$(NANVIXD_VMM_WARMSTART_BIN)$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/$(NANVIXD_VMM_WARMSTART_BIN).$(HOST_BIN_EXT)

check-nanvixd-vmm:
	@$(nanvixd_vmm_resolve_protoc); \
	PROTOC="$$PROTOC_BIN" $(HOST_CARGO_CHECK_CMD) -p nanvixd-vmm

format-nanvixd-vmm:
	$(HOST_CARGO_FMT_CMD) -p nanvixd-vmm

format-check-nanvixd-vmm:
	$(HOST_CARGO_FMT_CMD) -p nanvixd-vmm --check

clean-nanvixd-vmm:
	$(HOST_CARGO_CLEAN_CMD) -p nanvixd-vmm
	$(RM_CMD) $(BINARIES_DIR)/$(NANVIXD_VMM_BIN).$(HOST_BIN_EXT)
	$(RM_CMD) $(BINARIES_DIR)/$(NANVIXD_VMM_WARMSTART_BIN).$(HOST_BIN_EXT)

rust-lint-nanvixd-vmm:
	@$(nanvixd_vmm_resolve_protoc); \
	PROTOC="$$PROTOC_BIN" $(HOST_CARGO_CLIPPY_CMD) --tests -p nanvixd-vmm --fix --allow-dirty --allow-no-vcs

rust-lint-check-nanvixd-vmm:
	@$(nanvixd_vmm_resolve_protoc); \
	PROTOC="$$PROTOC_BIN" $(HOST_CARGO_CLIPPY_CMD) --tests -p nanvixd-vmm -- -D warnings
