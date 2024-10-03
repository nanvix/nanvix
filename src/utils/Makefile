# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-loader all-echo-client

clean: clean-loader clean-echo-client

clippy: clippy-loader clippy-echo-client

all-echo-client:
	$(MAKE) -C echo-client

clean-echo-client:
	$(MAKE) -C echo-client clean

clippy-echo-client:
	$(MAKE) -C echo-client clippy

all-loader:
	$(MAKE) -C loader

clean-loader:
	$(MAKE) -C loader clean

clippy-loader:
	$(MAKE) -C loader clippy
