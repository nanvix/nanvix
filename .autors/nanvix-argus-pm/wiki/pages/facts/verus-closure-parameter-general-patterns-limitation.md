---
id: verus-closure-parameter-general-patterns-limitation
type: fact
status: stable
title: Verus rejects non-variable closure parameters
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

Verus `0.2026.08.23.fbbbbcf` rejects any closure whose **parameter** is a
non-variable (general) pattern — tuple `|(a, b)|`, reference `|&x|`, nested
`|&(_, b)|`, or wildcard `|_|` — during frontend lowering with:

```text
The verifier does not yet support the following Rust feature:
only variables are supported here, not general patterns
```

This is a HIR→VIR closure-parameter lowering limitation; it fires before any
verification runs and is independent of contracts, invariants, lemmas, or SMT
difficulty. The same tuple/reference/wildcard patterns are accepted by Verus in
`let` bindings and in `for`-loop patterns (`for (i, _) in xs.iter().enumerate()`
lowers fine) — the limitation is specific to closure parameters.

The behavior-equivalent form binds a single named parameter and recovers each
value by field access or dereference in the body:

- `|(a, _)| *a`            → `|p| p.0`
- `|(a, b)| f(a, b)`       → `|p| f(p.0, p.1)`
- `|&x| g(x)`              → `|x| g(*x)`
- `|_| v`                  → `|_idx| v`   (underscore-prefixed name, not `_`)
- `|&k, v| h(k, v)`        → `|k, v| h(*k, v)`

Under match ergonomics the original destructured bindings are references
(`a: &T`), so the named-parameter projections reproduce the exact types; when the
projected/dereferenced types are `Copy` the field reads copy exactly as the
original derefs did. Evaluation order, borrows, moves, return values, side
effects, error paths, unsafe behavior, and concurrency are unchanged.

In `src/kernel/src/pm/**`, the run-4 baseline probe attributed 10 diagnostics to
this frontend limitation across six files. Rewriting the closure parameters (16
in total: the 10 reported sites plus 6 same-family sites masked by
`--multiple-errors 4`) removed the family completely (10 → 0) while leaving every
other diagnostic family unchanged and exposing no new family. The post-rewrite
66-file layered probe (`run-5`) reached a fixed point, the kernel still compiled
under the x86-kernel target with `microvm,trace`, and all instrumented files were
restored byte-for-byte. This is a layered-probe fixed point for the bounded
family, not global PM completion: 38 diagnostics from other families remained.

The durable run record is in `research/GROUND_TRUTH.md`; the minimized examples,
run-5 structured result and family delta, launch contract, and restoration report
are under `/home/ruize/argus-pm-artifacts-20260824/`.
