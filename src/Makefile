# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-benchmarks all-daemons all-kernel all-microvm

clean: clean-benchmarks clean-daemons clean-kernel clean-microvm

clippy: clippy-benchmarks clippy-daemons

all-benchmarks:
	$(MAKE) -C benchmarks

clean-benchmarks:
	$(MAKE) -C benchmarks clean

clippy-benchmarks:
	$(MAKE) -C benchmarks clippy

all-daemons:
	$(MAKE) -C daemons

clean-daemons:
	$(MAKE) -C daemons clean

clippy-daemons:
	$(MAKE) -C daemons clippy

all-kernel:
	$(MAKE) -C kernel

clean-kernel:
	$(MAKE) -C kernel clean

all-microvm:
	$(MAKE) -C microvm all-microvm

clean-microvm:
	$(MAKE) -C microvm clean-microvm
