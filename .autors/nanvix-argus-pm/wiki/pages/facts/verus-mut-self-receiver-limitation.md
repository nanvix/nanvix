---
id: verus-mut-self-receiver-limitation
type: fact
status: stable
title: Verus rejects by-value mutable receivers
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

Verus `0.2026.08.23.fbbbbcf` rejects an ordinary-Rust method receiver written
as `fn f(mut self)` during frontend lowering with:

```text
The verifier does not yet support the following Rust feature: mut self
```

The behavior-equivalent form `fn f(self) { let mut this = self; ... }` lowers
past the frontend. Both forms retain by-value ownership; binding mutability is
not part of the method interface or ABI.

In `src/kernel/src/pm/**`, the full baseline probe attributed 20 diagnostics to
this frontend limitation. Rewriting those 20 methods removed the family
completely while preserving every one of the other 48 diagnostic rows. The
post-rewrite 66-file layered probe reached a fixed point, and all instrumented
source bytes were restored.

Evidence is recorded in:

- `/home/ruize/argus-pm-artifacts-20260824/reproducers/REPRODUCER_EVIDENCE.md`
- `/home/ruize/argus-pm-artifacts-20260824/probe/pm_baseline_run-3.json`
- `/home/ruize/argus-pm-artifacts-20260824/probe/pm_acceptance_run-4.json`
- `/home/ruize/argus-pm-artifacts-20260824/COMMIT_MATERIAL_mut_self.md`
