# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-c-bindings all-network-c all-thread-c

clean: clean-c-bindings clean-network-c clean-thread-c

all-c-bindings:
	$(MAKE) -C c-bindings

clean-c-bindings:
	$(MAKE) -C c-bindings clean

all-thread-c:
	$(MAKE) -C thread-c

clean-thread-c:
	$(MAKE) -C thread-c clean

all-network-c:
	$(MAKE) -C test-network-c

clean-network-c:
	$(MAKE) -C test-network-c clean
