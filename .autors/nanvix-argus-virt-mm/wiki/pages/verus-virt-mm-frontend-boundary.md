---
id: verus-virt-mm-frontend-boundary
type: fact
status: stable
title: Verus frontend boundary in Nanvix virtual memory
sources: []
created_at: 2026-08-24T15:20:47+08:00
last_reviewed_at: 2026-08-24T15:20:47+08:00
reviewer_note: Reviewer-certified from the canonical run-6 evidence.
---

# Verus frontend boundary in Nanvix virtual memory

The audited x86-kernel virtual-memory scope contains 30 scanned Rust files, of
which 29 are instrumented. The platform microvm module is scan-only to avoid
introducing unrelated platform-global diagnostics. Scope edges and exclusions
are documented in `research/GROUND_TRUTH.md`.

On Verus `0.2026.08.23.fbbbbcf`, two diagnostic categories are established
frontend limitations by minimized legal-Rust reproducers:

- Five project occurrences of `static mut` are rejected by the Verus frontend.
  They are coupled to global ownership and require architecture-level treatment,
  not a local trusted shim.
- A project-owned cross-crate trait implementation with associated constants
  causes generated `VERUS_UNERASED_PROXY__*` members to be rejected with E0407.
  Adding `external_body` to the project-owned implementation would be an
  impermissible trust expansion.

The accepted reproducer evidence is in `research/reproducers/RESULTS.md`.

Canonical layered probing can peel the injected markers from the five static
declarations and the associated-constant implementation. This exposes a deeper
frontier but does not establish a fixed point: the run stops at
`unresolved_frontier` after four rounds with project-function, project-type,
type-specification, assume-specification, and opaque-type obligations. Those
obligations are not evidence of frontend limitations.

Diagnostic cardinalities refer to different equivalence relations and must not
be compared as if they were the same count. Run-6 has 164 aggregated raw
diagnostic records, 152 unique diagnostic-family/source-site occurrences after
exact repeat collapse, and 142 source-stable baseline identities after message
normalization. All 152 family/site occurrences are classified; all 142
source-stable identities are injection-induced relative to a pristine baseline
with zero diagnostics.

Thirty-six additional frontend-looking occurrences remain candidates rather
than established limitations because they lack minimized reproductions on the
pinned Verus build. Seven are static-path accesses coupled to the known global
ownership architecture; the remaining candidates include pointer casts,
pattern restrictions, inline assembly, const blocks, panic expansion, pointer
dereference, complex break expressions, and mutable-reference closure capture.

Canonical evidence is retained under
`/home/ruize/argus-virt-mm-artifacts-20260824/probe_run6/`, especially
`triage.json`, `triage.md`, `baseline_reconcile.json`,
`restoration_check.txt`, and `progress.jsonl`.
