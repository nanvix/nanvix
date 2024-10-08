# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

all: all-benchmarks all-daemons all-kernel all-libs all-microvm all-user all-utils

clean: clean-benchmarks clean-daemons clean-kernel clean-libs clean-microvm clean-user clean-utils

clippy: clippy-benchmarks clippy-daemons clippy-libs clippy-user clippy-utils

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

all-libs:
	$(MAKE) -C libs

clean-libs:
	$(MAKE) -C libs clean

clippy-libs:
	$(MAKE) -C libs clippy

all-microvm:
	$(MAKE) -C microvm all-microvm

clean-microvm:
	$(MAKE) -C microvm clean-microvm

all-utils:
	$(MAKE) -C utils

clean-utils:
	$(MAKE) -C utils clean

clippy-utils:
	$(MAKE) -C utils clippy

all-user:
	$(MAKE) -C user

clean-user:
	$(MAKE) -C user clean

clippy-user:
	$(MAKE) -C user clippy
