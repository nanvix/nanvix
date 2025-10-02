# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-echo-c all-echo-cpp all-noop-c all-noop-cpp

clean: clean-echo-c clean-echo-cpp clean-noop-c clean-noop-cpp

all-echo-c:
	$(MAKE) -C echo-c

clean-echo-c:
	$(MAKE) -C echo-c clean

all-echo-cpp:
	$(MAKE) -C echo-cpp

clean-echo-cpp:
	$(MAKE) -C echo-cpp clean

all-noop-c:
	$(MAKE) -C noop-c

clean-noop-c:
	$(MAKE) -C noop-c clean

all-noop-cpp:
	$(MAKE) -C noop-cpp

clean-noop-cpp:
	$(MAKE) -C noop-cpp clean
