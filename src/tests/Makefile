# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-c-bindings all-dlfcn-c all-file-c all-memory all-misc-c all-network-c all-thread-c

clean: clean-c-bindings clean-dlfcn-c clean-file-c clean-memory clean-misc-c clean-network-c clean-thread-c

all-c-bindings:
	$(MAKE_QUIET) -C c-bindings

clean-c-bindings:
	$(MAKE_QUIET) -C c-bindings clean

all-dlfcn-c:
	$(MAKE_QUIET) -C dlfcn-c

clean-dlfcn-c:
	$(MAKE_QUIET) -C dlfcn-c clean

all-file-c:
	$(MAKE_QUIET) -C file-c

clean-file-c:
	$(MAKE_QUIET) -C file-c clean

all-thread-c:
	$(MAKE_QUIET) -C thread-c

clean-thread-c:
	$(MAKE_QUIET) -C thread-c clean

all-memory:
	$(MAKE_QUIET) -C memory-c

clean-memory:
	$(MAKE_QUIET) -C memory-c clean

all-misc-c:
	$(MAKE_QUIET) -C misc-c

clean-misc-c:
	$(MAKE_QUIET) -C misc-c clean

all-network-c:
	$(MAKE_QUIET) -C network-c

clean-network-c:
	$(MAKE_QUIET) -C network-c clean
