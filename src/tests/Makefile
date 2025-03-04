# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-c-bindings all-thread-c all-thread-mutex-c all-thread-nowait-c

clean: clean-c-bindings clean-thread-c clean-thread-mutex-c clean-thread-nowait-c

all-c-bindings:
	$(MAKE) -C c-bindings

clean-c-bindings:
	$(MAKE) -C c-bindings clean

all-thread-c:
	$(MAKE) -C thread-c

clean-thread-c:
	$(MAKE) -C thread-c clean

all-thread-mutex-c:
	$(MAKE) -C thread-mutex-c

clean-thread-mutex-c:
	$(MAKE) -C thread-mutex-c clean

all-thread-nowait-c:
	$(MAKE) -C thread-nowait-c

clean-thread-nowait-c:
	$(MAKE) -C thread-nowait-c clean
