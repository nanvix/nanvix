---
id: virt-mm-workspace-verification-boundaries
type: fact
status: stable
title: Workspace reachability does not imply Verus crate verification
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
tags:
  - nanvix
  - virtual-memory
  - verus
  - dependency-closure
related_projects:
  - nanvix-argus-virt-mm
---

Nanvix verifies only the crates listed in the root Makefile's `VERUS_CRATES`
variable in their own crate contexts. For the audited i686 virtual-memory
closure, `arch`, `bitmap`, and `sys` are both reached and listed verification
targets. `nanvix-slab` is a verification target but is not reached by this
closure.

The reached project-owned crates `config`, `bump_allocator`, `elf`, `error`, and
`raw-array` are not listed verification targets. The kernel invocation compiles
them as ordinary dependencies, but the project pipeline does not independently
Verus-verify their executable bodies. This distinction explains why
`bump_allocator::BssStorage` is seen as a non-Verus-processed cross-crate trait
when the kernel's `PageTableBss` implementation triggers the associated-constant
proxy limitation.

The `config::kernel::MEMORY_SIZE` dependency also crosses a generated-source
chain that source reachability does not expose directly:
`src/libs/config/src/lib.rs` includes an `OUT_DIR` `kernel_config.rs` generated
by `src/libs/config/build.rs`, which obtains the value through
`src/libs/build-utils/src/lib.rs`. The generated file and host build-script code
are project-owned dependency boundaries, not kernel scan/injection inputs and
not external or hardware trust.

Evidence:

- `Makefile`
- `src/kernel/src/mm/virt/identity_map.rs`
- `src/kernel/src/mm/virt/page_table_allocator.rs`
- `src/libs/config/src/lib.rs`
- `src/libs/config/build.rs`
- `src/libs/build-utils/src/lib.rs`
- `src/libs/bump_allocator/src/lib.rs`
- `/home/ruize/argus-virt-mm-artifacts-20260824/scope/scope_manifest.json`
