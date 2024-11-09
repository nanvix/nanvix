# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-boottime all-echo all-linux-app all-perf

clean: clean-boottime clean-echo clean-linux-app clean-perf

clippy: clippy-boottime clippy-echo clippy-linux-app clippy-perf

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

all-linux-app:
	$(MAKE) -C linux-app all

clean-linux-app:
	$(MAKE) -C linux-app clean

clippy-linux-app:
	$(MAKE) -C linux-app clippy

all-perf:
	$(MAKE) -C perf all

clean-perf:
	$(MAKE) -C perf clean

clippy-perf:
	$(MAKE) -C perf clippy
