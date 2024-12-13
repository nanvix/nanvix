# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-procd all-linuxd

clean: clean-procd clean-linuxd

# TODO: clippy-linuxd
clippy: clippy-procd

all-procd:
	$(MAKE) -C procd

clean-procd:
	$(MAKE) -C procd clean

clippy-procd:
	$(MAKE) -C procd clippy

all-linuxd:
	$(MAKE) -C linuxd

clean-linuxd:
	$(MAKE) -C linuxd clean

clippy-linuxd:
	$(MAKE) -C linuxd clippy
