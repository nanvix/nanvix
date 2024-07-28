# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-initd all-memd all-test

clean: clean-initd clean-memd clean-test

all-initd:
	$(MAKE) -C initd all

clean-initd:
	$(MAKE) -C initd clean

all-memd:
	$(MAKE) -C memd all

clean-memd:

all-test:
	$(MAKE) -C test all

clean-test:
	$(MAKE) -C test clean
