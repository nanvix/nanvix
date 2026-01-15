# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Main target for running all nanvixd tests in both HTTP and terminal modes.
run-nanvixd-tests: | run-nanvixd-http-tests run-nanvixd-terminal-tests

# HTTP mode tests: These tests run programs via nanvixd's HTTP API.
# Program arguments and input are passed as JSON payloads.
run-nanvixd-http-tests: | \
	init-repo \
	test-nanvixd-http-dlfcn-c \
	test-nanvixd-http-echo-c \
	test-nanvixd-http-echo-cpp \
	test-nanvixd-http-echo-rust-nostd \
	test-nanvixd-http-echo-wasm-rust \
	test-nanvixd-http-file-c \
	test-nanvixd-http-file-rust \
	test-nanvixd-http-thread-rust \
	test-nanvixd-http-stress-rust \
	test-nanvixd-http-hello-c \
	test-nanvixd-http-hello-cpp \
	test-nanvixd-http-hello-wasm \
	test-nanvixd-http-linux-app \
	test-nanvixd-http-memory-c \
	test-nanvixd-http-misc-c \
	test-nanvixd-http-network-c \
	test-nanvixd-http-python3 \
	test-nanvixd-http-qjs \
	test-nanvixd-http-quickjs \
	test-nanvixd-http-arch-rust \
	test-nanvixd-http-thread-c

# Terminal mode tests: These tests run programs directly via nanvixd's terminal interface.
# Input is provided via stdin, and WASM/QuickJS tests are not supported (requires HTTP mode).
run-nanvixd-terminal-tests: | \
	init-repo \
	test-nanvixd-terminal-echo-c \
	test-nanvixd-terminal-echo-cpp \
	test-nanvixd-terminal-echo-rust-nostd \
	test-nanvixd-terminal-hello-c \
	test-nanvixd-terminal-hello-cpp \
	test-nanvixd-terminal-linux-app \
	test-nanvixd-terminal-dlfcn-c \
	test-nanvixd-terminal-file-c \
	test-nanvixd-terminal-file-rust \
	test-nanvixd-terminal-thread-rust \
	test-nanvixd-terminal-stress-rust \
	test-nanvixd-terminal-thread-c \
	test-nanvixd-terminal-network-c \
	test-nanvixd-terminal-misc-c \
	test-nanvixd-terminal-memory-c \
	test-nanvixd-terminal-arch-rust

include build/make/test/system.mk
include build/make/test/wasm.mk
include build/make/test/quickjs.mk
