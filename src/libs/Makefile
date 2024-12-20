# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-nvx all-procd all-posix

clean: clean-nvx clean-procd clean-posix

# TODO: clippy-posix
clippy: clippy-nvx clippy-procd

all-nvx:
	$(MAKE) -C nvx

clean-nvx:
	$(MAKE) -C nvx clean

clippy-nvx:
	$(MAKE) -C nvx clippy

all-procd: all-nvx
	$(MAKE) -C procd

clean-procd:
	$(MAKE) -C procd clean

clippy-procd:
	$(MAKE) -C procd clippy

all-posix: all-nvx
	$(MAKE) -C posix

clean-posix:
	$(MAKE) -C posix clean

clippy-posix:
	$(MAKE) -C posix clippy
