# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-linuxd all-procd all-memd all-testd all-wasmd

clean: clean-linuxd clean-procd clean-memd clean-testd clean-wasmd

clippy: clippy-linuxd clippy-procd clippy-memd clippy-testd clippy-wasmd

all-linuxd:
	$(MAKE) -C linuxd all

clean-linuxd:
	$(MAKE) -C linuxd clean

clippy-linuxd:
	$(MAKE) -C linuxd clippy

all-procd:
	$(MAKE) -C procd all

clean-procd:
	$(MAKE) -C procd clean

clippy-procd:
	$(MAKE) -C procd clippy

all-memd:
	$(MAKE) -C memd all

clean-memd:
	$(MAKE) -C memd clean

clippy-memd:
	$(MAKE) -C memd clippy

all-testd:
	$(MAKE) -C testd all

clean-testd:
	$(MAKE) -C testd clean

clippy-testd:
	$(MAKE) -C testd clippy

all-wasmd:
	$(MAKE) -C wasmd all

clean-wasmd:
	$(MAKE) -C wasmd clean

clippy-wasmd:
	$(MAKE) -C wasmd clippy
