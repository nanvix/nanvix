# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-hello all-hello-rust

clean: clean-hello clean-hello-rust

clippy: clippy-hello clippy-hello-rust

all-hello:
	$(MAKE) -C hello

clean-hello:
	$(MAKE) -C hello clean

clippy-hello:
	$(MAKE) -C hello clippy

all-hello-rust:
	$(MAKE) -C hello-rust

clean-hello-rust:
	$(MAKE) -C hello-rust clean

clippy-hello-rust:
	$(MAKE) -C hello-rust clippy
