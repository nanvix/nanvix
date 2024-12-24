# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-hello-wasm all-hello-rust all-hello-c all-hello-cpp

clean: clean-hello-wasm clean-hello-rust clean-hello-c clean-hello-cpp

clippy: clippy-hello-wasm clippy-hello-rust

all-hello-wasm:
	$(MAKE) -C hello-wasm

clean-hello-wasm:
	$(MAKE) -C hello-wasm clean

clippy-hello-wasm:
	$(MAKE) -C hello-wasm clippy

all-hello-rust:
	$(MAKE) -C hello-rust

clean-hello-rust:
	$(MAKE) -C hello-rust clean

clippy-hello-rust:
	$(MAKE) -C hello-rust clippy

all-hello-c:
	$(MAKE) -C hello-c

clean-hello-c:
	$(MAKE) -C hello-c clean

all-hello-cpp:
	$(MAKE) -C hello-cpp

clean-hello-cpp:
	$(MAKE) -C hello-cpp clean
