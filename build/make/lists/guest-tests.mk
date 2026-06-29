# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Guest integration/system test binaries. One entry per line to minimize merge
# conflicts between branches that each add a test.
ALL_GUEST_TESTS := \
	test-rust-testd \
	test-rust-file \
	test-rust-fork-guestfs \
	test-rust-fork-hostfs \
	test-rust-fork-kcall \
	test-rust-waitpid \
	test-rust-kill \
	test-rust-job-control \
	test-rust-setenv \
	test-rust-thread \
	test-rust-stress \
	test-rust-kernel \
	test-rust-mmio-fault \
	test-rust-sigsegv \
	test-rust-linux-app \
	test-rust-arch \
	test-rust-vfs-test \
	test-rust-misc \
	test-rust-memory \
	test-rust-network \
	test-rust-c-bindings \
	test-rust-mount-test \
	test-rust-mount-multipart-test \
	test-rust-cmdline-len \
	test-rust-env-nostd \
	test-rust-cmdline-env-nostd \
	test-rust-getenv-nostd \
	test-rust-snapshot-test \
	test-rust-execv-test \
	test-rust-execv-target \
	test-rust-execv-big-target \
	test-rust-pipe-dup2 \
	test-rust-fork-exec-vfsd-test \
	test-rust-fork-exec-vfsd-target \
	test-rust-fork-exec-write-test \
	test-rust-fork-exec-write-target \
	test-rust-thread-vfs-test \
	test-rust-fork-exec-loop-test \
	test-rust-fork-exec-loop-target \
	test-rust-socket-fork \
	test-rust-fork-exec-pipe-bulk-test \
	test-rust-fork-exec-pipe-bulk-target \
	test-rust-fork-exec-pipe-loop-test \
	test-rust-fork-exec-pipe-loop-target \
	test-rust-fork-exec-argv-space-test \
	test-rust-fork-exec-argv-space-target
# dlfcn-rust requires PIE linking for dlopen/dlsym; the x86_64 static
# relocation model produces R_X86_64_32 relocations incompatible with PIE.
ifneq ($(TARGET),x86_64)
ALL_GUEST_TESTS += test-rust-dlfcn
endif
