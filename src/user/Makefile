# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-hello all-hello-rust all-hello-c all-hello-cpp

clean: clean-hello clean-hello-rust clean-hello-c clean-hello-cpp

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

all-hello-c:
	$(MAKE) -C hello-c

clean-hello-c:
	$(MAKE) -C hello-c clean

all-hello-cpp:
	$(MAKE) -C hello-cpp

clean-hello-cpp:
	$(MAKE) -C hello-cpp clean
