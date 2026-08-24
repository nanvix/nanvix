---
id: virt-mm-platform-dependencies
type: fact
status: stable
title: Virt-MM directly depends on microvm platform helpers
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
tags:
  - nanvix
  - virtual-memory
  - dependency-closure
related_projects:
  - nanvix-argus-virt-mm
---

The x86 microvm virtual-memory implementation has direct project-owned
dependencies in `src/kernel/src/hal/platform/microvm/mod.rs`:

- `src/kernel/src/mm/virt/page_table_allocator.rs` reads
  `crate::hal::platform::NUM_PAGE_TABLES`.
- `src/kernel/src/mm/virt/vmem.rs` calls
  `crate::hal::platform::is_valid_physical_region` and
  `crate::hal::platform::virt_to_phys`.

Consequently, a virt-MM dependency manifest that omits the microvm platform
implementation is incomplete. Because the platform file also contains unrelated
initialization and device state, a verifier probe should select the relevant
items rather than attribute every diagnostic in the file to virt-MM.

`vmem.rs` also names the x86_64 hardware-page-table module, but those calls are
guarded by `#[cfg(target_arch = "x86_64")]` and are inactive for
`build/targets/x86-kernel.json`. They remain an explicit target-dependent
exclusion rather than an unrecorded missing edge.

Evidence: `src/kernel/src/mm/virt/page_table_allocator.rs`,
`src/kernel/src/mm/virt/vmem.rs`,
`src/kernel/src/hal/platform/mod.rs`, and
`src/kernel/src/hal/platform/microvm/mod.rs`. The initial candidate manifest is
stored at `/home/ruize/argus-virt-mm-artifacts-20260824/scope/scope_manifest.json`.
