# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-init all-memd all-test

clean: clean-init clean-memd clean-test

all-init:
	$(MAKE) -C init all

clean-init:
	$(MAKE) -C init clean

all-memd:
	$(MAKE) -C memd all

clean-memd:

all-test:
	$(MAKE) -C test all

clean-test:
	$(MAKE) -C test clean
