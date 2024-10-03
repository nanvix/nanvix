# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-benchmarks all-daemons all-kernel all-microvm all-user

clean: clean-benchmarks clean-daemons clean-kernel clean-microvm clean-user

clippy: clippy-benchmarks clippy-daemons clippy-user

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

all-user:
	$(MAKE) -C user

clean-user:
	$(MAKE) -C user clean

clippy-user:
	$(MAKE) -C user clippy
