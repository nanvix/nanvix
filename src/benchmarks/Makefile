# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-boottime all-echo

clean: clean-boottime clean-echo

clippy: clippy-boottime clippy-echo

all-echo:
	$(MAKE) -C echo all

clean-echo:
	$(MAKE) -C echo clean

clippy-echo:
	$(MAKE) -C echo clippy

all-boottime:
	$(MAKE) -C boottime all

clean-boottime:
	$(MAKE) -C boottime clean

clippy-boottime:
	$(MAKE) -C boottime clippy
