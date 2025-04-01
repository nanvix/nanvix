# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-c-bindings all-dlfcn-c all-file-c all-misc-c all-network-c all-thread-c

clean: clean-c-bindings clean-dlfcn-c clean-file-c clean-misc-c clean-network-c clean-thread-c

all-c-bindings:
	$(MAKE) -C c-bindings

clean-c-bindings:
	$(MAKE) -C c-bindings clean

all-dlfcn-c:
	$(MAKE) -C dlfcn-c

clean-dlfcn-c:
	$(MAKE) -C dlfcn-c clean

all-file-c:
	$(MAKE) -C file-c

clean-file-c:
	$(MAKE) -C file-c clean

all-thread-c:
	$(MAKE) -C thread-c

clean-thread-c:
	$(MAKE) -C thread-c clean

all-misc-c:
	$(MAKE) -C misc-c

clean-misc-c:
	$(MAKE) -C misc-c clean

all-network-c:
	$(MAKE) -C network-c

clean-network-c:
	$(MAKE) -C network-c clean
