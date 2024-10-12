# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-boottime all-echo all-linux-app

clean: clean-boottime clean-echo clean-linux-app

clippy: clippy-boottime clippy-echo clippy-linux-app

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
