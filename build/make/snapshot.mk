# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# The snapshots for the L2 VM need linuxd.elf to be built first.
all-snapshot: all-host-binaries
# Snapshots are only generated for microvm machines in L2 deployment mode.
ifneq (,$(and $(filter l2,$(DEPLOYMENT_MODE)),$(filter $(MACHINE),microvm)))
	bash $(SCRIPTS_DIR)/generate-l2-initramfs.sh
	bash $(SCRIPTS_DIR)/generate-l2-snapshot.sh $(CLH_DIR)
endif

clean-snapshot:
	$(FORCE_RM_CMD) $(SNAPSHOT_DIR)
