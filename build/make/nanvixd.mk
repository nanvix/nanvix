# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

NANVIXD_FEATURES :=
NANVIXD_FEATURES += $(if $(filter standalone,$(DEPLOYMENT_MODE)),standalone,)
NANVIXD_FEATURES += $(if $(filter single-process,$(DEPLOYMENT_MODE)),single-process,)
NANVIXD_FEATURES += $(if $(filter microvm,$(MACHINE)),microvm,)
NANVIXD_FEATURES += $(if $(filter yes,$(WHP)),whp,)
NANVIXD_FEATURES += $(if $(filter yes,$(PROFILER)),profile-time,)
NANVIXD_FEATURES += $(if $(filter yes,$(TIMESTAMP_MSG)),timestamp-messages,)
NANVIXD_FEATURES := $(strip $(NANVIXD_FEATURES))
NANVIXD_CARGO_FEATURES := $(if $(NANVIXD_FEATURES),--features "$(NANVIXD_FEATURES)")

# In standalone mode, nanvixd needs mkramfs to produce the rootfs image
# and guest binaries (which build shared libraries like libmul.so that
# are bundled into the standalone rootfs).
ifeq ($(DEPLOYMENT_MODE),standalone)
all-nanvixd: all-host-binaries-mkramfs all-guest-binaries
endif

# PDB filename for Windows symbol resolution (xperf, WPA, debuggers).
NANVIXD_PDB := nanvixd.pdb

all-nanvixd: init
	$(HOST_CARGO_BUILD_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd
	$(CP_CMD) $(OBJECTS_DIR)/$(BUILD_MODE)/nanvixd$(CARGO_EXE_SUFFIX) $(BINARIES_DIR)/nanvixd.$(HOST_BIN_EXT)
ifeq ($(IS_WINDOWS),yes)
	# Copy PDB alongside the exe for symbol resolution.
	@if [ -f "$(OBJECTS_DIR)/$(BUILD_MODE)/$(NANVIXD_PDB)" ]; then \
		cp -f "$(OBJECTS_DIR)/$(BUILD_MODE)/$(NANVIXD_PDB)" "$(BINARIES_DIR)/$(NANVIXD_PDB)"; \
	fi
endif
	# Build the standalone rootfs image from a seed directory using mkramfs.
ifeq ($(DEPLOYMENT_MODE),standalone)
	@mkdir -p $(BINARIES_DIR)/standalone-rootfs-seed/lib
	@mkdir -p $(BINARIES_DIR)/standalone-rootfs-seed/src
	@cp -f $(ROOT_DIR)/README.md $(BINARIES_DIR)/standalone-rootfs-seed/
	@if [ -f $(LIBRARIES_DIR)/libmul.so ]; then \
		cp -f $(LIBRARIES_DIR)/libmul.so $(BINARIES_DIR)/standalone-rootfs-seed/lib/; \
	fi
	@if [ -f $(LIBRARIES_DIR)/libmul-pie.so ]; then \
		cp -f $(LIBRARIES_DIR)/libmul-pie.so $(BINARIES_DIR)/standalone-rootfs-seed/lib/; \
	fi
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/standalone-rootfs.img $(BINARIES_DIR)/standalone-rootfs-seed/
	# Build the ramfs image for the execv() test. It contains the execv target program (built as a
	# guest binary, stripped at link time) at the filesystem root as "target", which test-rust-execv-test
	# opens and execs. Stripping keeps the on-disk image small.
	@mkdir -p $(BINARIES_DIR)/test-rust-execv-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-execv-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-execv-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-execv-test.img $(BINARIES_DIR)/test-rust-execv-test-seed/
	# Build the ramfs image for the execv() big-binary test. It contains the large target program
	# (inflated to MEMORY_SIZE/8) at the filesystem root as "target"; test-rust-execv-test execs it via the
	# same caller pointed at this image. This exercises execv() of a large binary with no size cap.
	@mkdir -p $(BINARIES_DIR)/test-rust-execv-big-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-execv-big-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-execv-big-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-execv-big-test.img $(BINARIES_DIR)/test-rust-execv-big-test-seed/
	# Build the ramfs image for the fork()+execv()+vfsd test. It contains the target program at the
	# filesystem root as "target"; test-rust-fork-exec-vfsd-test forks and the child execs it, after which the
	# target performs a vfsd read (the operation that hangs after fork()+execv()).
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-vfsd-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-vfsd-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-vfsd-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-vfsd-test.img $(BINARIES_DIR)/test-rust-fork-exec-vfsd-test-seed/
	# Build the ramfs image for the fork()+execv()+write test. It contains the target program at the
	# filesystem root as "target"; test-rust-fork-exec-write-test forks and the child execs it, after which the
	# target writes /exec_write.out -- a write whose visibility to the parent the caller checks.
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-write-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-write-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-write-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-write-test.img $(BINARIES_DIR)/test-rust-fork-exec-write-test-seed/
	# Build the ramfs image for the multi-threaded VFS attribution test (nanvix/nanvix#2529). It is
	# seeded with /input.dat (the payload a secondary thread reads through a descriptor the main
	# thread opened); the test creates /output.dat at runtime and reads it back.
	@mkdir -p $(BINARIES_DIR)/test-rust-thread-vfs-test-seed
	@printf 'THREAD-VFS-2529-PAYLOAD' > $(BINARIES_DIR)/test-rust-thread-vfs-test-seed/input.dat
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-thread-vfs-test.img $(BINARIES_DIR)/test-rust-thread-vfs-test-seed/
	# Build the ramfs image for the repeated fork()+execv() test. It contains the target at the
	# filesystem root as "target" and a seeded file "coldfile.dat" that the parent opens before each
	# fork; the child reads it through the inherited descriptor passed in argv.
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-loop-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-loop-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-loop-test-seed/target
	@printf 'COLD-READ-PAYLOAD-OK' > $(BINARIES_DIR)/test-rust-fork-exec-loop-test-seed/coldfile.dat
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-loop-test.img $(BINARIES_DIR)/test-rust-fork-exec-loop-test-seed/
	# Build the ramfs image for the bulk pipe integrity test. It contains the target at the
	# filesystem root as "target"; test-rust-fork-exec-pipe-bulk-test forks and the child dup2()s a pipe onto
	# standard output and execs it, after which the target streams a 1 MiB payload back through the
	# pipe -- a bulk transfer whose delivery in full to the parent the caller checks.
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-pipe-bulk-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-pipe-bulk-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-pipe-bulk-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-pipe-bulk-test.img $(BINARIES_DIR)/test-rust-fork-exec-pipe-bulk-test-seed/
	# Build the ramfs image for the repeated pipe-capture reliability test. It contains the target at
	# the filesystem root as "target"; test-rust-fork-exec-pipe-loop-test forks and the child dup2()s a pipe
	# onto standard output and execs it once per cycle, after which the target streams a payload back
	# through the pipe -- a capture whose full, repeated delivery to the parent the caller checks.
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-pipe-loop-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-pipe-loop-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-pipe-loop-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-pipe-loop-test.img $(BINARIES_DIR)/test-rust-fork-exec-pipe-loop-test-seed/
	# Build the ramfs image for the execv() space-argument test. It contains the target at the
	# filesystem root as "target"; test-rust-fork-exec-argv-space-test forks and the child execs it passing an
	# argument that contains an embedded space, which the target verifies arrives verbatim.
	@mkdir -p $(BINARIES_DIR)/test-rust-fork-exec-argv-space-test-seed
	@cp -f $(BINARIES_DIR)/test-rust-fork-exec-argv-space-target.$(EXEC_FORMAT) $(BINARIES_DIR)/test-rust-fork-exec-argv-space-test-seed/target
	$(BINARIES_DIR)/mkramfs.$(HOST_BIN_EXT) -o $(BINARIES_DIR)/test-rust-fork-exec-argv-space-test.img $(BINARIES_DIR)/test-rust-fork-exec-argv-space-test-seed/
endif

check-nanvixd:
	@$(HOST_CARGO_CHECK_CMD) $(NANVIXD_CARGO_FEATURES) -p nanvixd

format-nanvixd:
	$(HOST_CARGO_FMT_CMD) -p nanvixd

format-check-nanvixd:
	$(HOST_CARGO_FMT_CMD) -p nanvixd --check

clean-nanvixd:
	$(HOST_CARGO_CLEAN_CMD) -p nanvixd
	$(RM_CMD) $(BINARIES_DIR)/nanvixd.$(HOST_BIN_EXT)

rust-lint-nanvixd:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIXD_CARGO_FEATURES) -p nanvixd --fix --allow-dirty --allow-no-vcs

rust-lint-check-nanvixd:
	$(HOST_CARGO_CLIPPY_CMD) --tests $(NANVIXD_CARGO_FEATURES) -p nanvixd -- -D warnings
