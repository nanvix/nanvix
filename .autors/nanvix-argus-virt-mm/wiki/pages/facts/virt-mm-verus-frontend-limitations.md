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
reviewer_note: Confirmed by full-verification probes, native kernel verification, and minimized latest-Verus reproducers.
---

The audited i686 `microvm trace` virtual-memory closure exposes two Verus
frontend limitation categories before SMT verification begins:

- Verus rejects five project `static mut` storage sites used by virtual and
  physical memory management when the probe injects `#[verus_verify]`. The
  current declaration-aware peeler gives each item a distinct AST identity and
  removes its injected marker, so these items no longer terminate the layered
  run.
- Verus emits E0407 for the two associated constants in
  `impl bump_allocator::BssStorage for PageTableBss`. `BssStorage` is
  project-owned but is consumed as a cross-crate, non-Verus-processed trait by
  the kernel invocation. The peeler resolves member diagnostics to the enclosing
  named `BssStorage` impl and removes every injected marker in that impl; the
  limitation does not make the implementation an acceptable trusted boundary.

Both categories reproduce in minimized legal Rust on Verus
`0.2026.08.23.fbbbbcf`. Native `make verify-kernel` succeeds with 1882 kernel
items verified and zero errors because these declarations are outside the
current native verification surface. The repaired canonical probe reaches four
rounds instead of the former two-round frontier, then stops on
project/type/specification and opaque-type obligations. This is deeper coverage,
not a complete fixed point.

Evidence:

- `research/reproducers/RESULTS.md`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/findings.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/triage.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run3/baseline_reconcile.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run5/findings.json`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run5/restoration_check.txt`
- `/home/ruize/argus-virt-mm-artifacts-20260824/probe_run5/make_verify_kernel.stdout.log`
