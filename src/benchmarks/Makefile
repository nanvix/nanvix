# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-echo-c all-echo-cpp all-echo-wasm-js all-noop-c all-noop-cpp all-noop-wasm-js

clean: clean-echo-c clean-echo-cpp all-echo-wasm-js clean-noop-c clean-noop-cpp clean-noop-wasm-js

all-echo-c:
	$(MAKE) -C echo-c

clean-echo-c:
	$(MAKE) -C echo-c clean

all-echo-cpp:
	$(MAKE) -C echo-cpp

clean-echo-cpp:
	$(MAKE) -C echo-cpp clean

all-echo-wasm-js:
	$(MAKE) -C echo-wasm-js

clean-echo-wasm-js:
	$(MAKE) -C echo-wasm-js clean

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
