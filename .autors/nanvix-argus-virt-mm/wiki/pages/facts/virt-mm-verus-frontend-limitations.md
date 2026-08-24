---
id: virt-mm-verus-frontend-limitations
type: fact
status: stable
title: Verus frontend blockers in the Nanvix i686 virtual-memory closure
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
related_projects:
  - nanvix-argus-virt-mm
reviewer_note: Confirmed by a full-verification probe and minimized latest-Verus reproducers.
---

The audited i686 `microvm trace` virtual-memory closure exposes two Verus
frontend limitation categories before SMT verification begins:

- Verus rejects five project `static mut` storage sites used by virtual and
  physical memory management. These non-function items cannot be peeled by the
  current function-frontier mechanism, so the layered run terminates with
  `stop_reason=unresolved_frontier` and `layered_complete=false`.
- Verus emits E0407 for the two associated constants in
  `impl bump_allocator::BssStorage for PageTableBss`. `BssStorage` is
  project-owned but is consumed as a cross-crate, non-Verus-processed trait by
  the kernel invocation; the limitation does not make the implementation an
  acceptable trusted boundary.

Both categories reproduce in minimized legal Rust on Verus
`0.2026.08.23.fbbbbcf`. The incomplete layered run does not establish that no
deeper frontend limitations exist behind these blockers.

Evidence:

- `research/reproducers/RESULTS.md`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/findings.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/triage.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/baseline_reconcile.json`
