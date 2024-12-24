# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-loader all-echo-client all-nanvixd

clean: clean-loader clean-echo-client clean-nanvixd

clippy: clippy-loader clippy-echo-client clippy-nanvixd

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

all-nanvixd:
	$(MAKE) -C nanvixd

clean-nanvixd:
	$(MAKE) -C nanvixd clean

clippy-nanvixd:
	$(MAKE) -C nanvixd clippy
