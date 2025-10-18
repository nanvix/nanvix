# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# The snapshots for the L2 VM need linuxd.elf to be built first.
all-snapshot: all-host-binaries
# Snapshots are only generated for microvm/hyperlight machines when L2_VM is enabled.
ifneq (,$(and $(filter yes,$(L2_VM)),$(filter $(MACHINE),microvm hyperlight)))
	bash $(SCRIPTS_DIR)/generate-l2-initramfs.sh
	bash $(SCRIPTS_DIR)/generate-l2-snapshot.sh $(TOOLCHAIN_DIR)
endif

clean-snapshot:
	$(FORCE_RM_CMD) $(SNAPSHOT_DIR)
