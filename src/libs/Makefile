# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-procd all-posix

clean: clean-procd clean-posix

# TODO: clippy-posix
clippy: clippy-procd

all-procd:
	$(MAKE) -C procd

clean-procd:
	$(MAKE) -C procd clean

clippy-procd:
	$(MAKE) -C procd clippy

all-posix:
	$(MAKE) -C posix

clean-posix:
	$(MAKE) -C posix clean

clippy-posix:
	$(MAKE) -C posix clippy
