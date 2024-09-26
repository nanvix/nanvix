# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-benchmarks all-daemons all-microvm

clean: clean-benchmarks clean-daemons clean-microvm

all-benchmarks:
	$(MAKE) -C benchmarks

clean-benchmarks:
	$(MAKE) -C benchmarks clean

all-daemons:
	$(MAKE) -C daemons

clean-daemons:
	$(MAKE) -C daemons clean

all-microvm:
	$(MAKE) -C microvm

clean-microvm:
	$(MAKE) -C microvm clean
