# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-echo-c all-noop-c all-noop-cpp all-noop-wasm-js

clean: clean-echo-c clean-noop-c clean-noop-cpp clean-noop-wasm-js

all-echo-c:
	$(MAKE) -C echo-c

clean-echo-c:
	$(MAKE) -C echo-c clean

all-noop-c:
	$(MAKE) -C noop-c

clean-noop-c:
	$(MAKE) -C noop-c clean

all-noop-cpp:
	$(MAKE) -C noop-cpp

clean-noop-cpp:
	$(MAKE) -C noop-cpp clean

all-noop-wasm-js:
	$(MAKE) -C noop-wasm-js

clean-noop-wasm-js:
	$(MAKE) -C noop-wasm-js clean
