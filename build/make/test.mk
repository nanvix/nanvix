# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

run-nanvixd-tests: | \
	init-repo \
	test-dlfcn-c \
	test-echo-c \
	test-echo-cpp \
	test-echo-rust-nostd \
	test-echo-wasm-rust \
	test-file-c \
	test-file-rust \
	test-hello-c \
	test-hello-cpp \
	test-hello-wasm \
	test-linux-app \
	test-memory-c \
	test-misc-c \
	test-network-c \
	test-python3 \
	test-qjs \
	test-quickjs \
	test-arch-rust \
	test-thread-c

include build/make/test/system.mk
include build/make/test/wasm.mk
include build/make/test/quickjs.mk
