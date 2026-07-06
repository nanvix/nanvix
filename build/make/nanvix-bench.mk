# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

NANVIX_BENCH_FEATURES :=
NANVIX_BENCH_FEATURES += profile-time
NANVIX_BENCH_FEATURES += $(if $(filter standalone,$(DEPLOYMENT_MODE)),standalone,)
NANVIX_BENCH_FEATURES += $(if $(filter single-process,$(DEPLOYMENT_MODE)),single-process,)
NANVIX_BENCH_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
NANVIX_BENCH_FEATURES += $(if $(filter yes,$(WHP)),whp,)
NANVIX_BENCH_FEATURES += $(if $(filter yes,$(TIMESTAMP_MSG)),timestamp-messages,)
NANVIX_BENCH_FEATURES := $(strip $(NANVIX_BENCH_FEATURES))
NANVIX_BENCH_CARGO_FEATURES := $(if $(NANVIX_BENCH_FEATURES),--features "$(NANVIX_BENCH_FEATURES)")

all-nanvix-bench: init
	$(HOST_CARGO_BUILD_CMD) $(NANVIX_BENCH_CARGO_FEATURES) -p nanvix-bench
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/nanvix-bench$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/nanvix-bench.$(HOST_BIN_EXT)

check-nanvix-bench:
	@$(HOST_CARGO_CHECK_CMD) $(NANVIX_BENCH_CARGO_FEATURES) -p nanvix-bench

format-nanvix-bench:
	$(HOST_CARGO_FMT_CMD) -p nanvix-bench

format-check-nanvix-bench:
	$(HOST_CARGO_FMT_CMD) -p nanvix-bench --check

clean-nanvix-bench:
	$(HOST_CARGO_CLEAN_CMD) -p nanvix-bench
	$(RM_CMD) $(BINARIES_DIR)/nanvix-bench.$(HOST_BIN_EXT)

rust-lint-nanvix-bench:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIX_BENCH_CARGO_FEATURES) -p nanvix-bench --fix --allow-dirty --allow-no-vcs

rust-lint-check-nanvix-bench:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIX_BENCH_CARGO_FEATURES) -p nanvix-bench -- -D warnings
