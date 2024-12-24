# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Builds everything.
all: all-hello-world all-matrix all-noop

# Cleans everything.
clean: clean-hello-world clean-matrix clean-noop

# Builds hello-world image.
all-hello-world:
	$(MAKE) -C hello-world all

# Cleans hello-world image.
clean-hello-world:
	$(MAKE) -C hello-world clean

# Builds matrix image.
all-matrix:
	$(MAKE) -C matrix all

# Cleans matrix image.
clean-matrix:
	$(MAKE) -C matrix clean

# Builds noop image.
all-noop:
	$(MAKE) -C noop all

# Cleans noop image.
clean-noop:
	$(MAKE) -C noop clean
