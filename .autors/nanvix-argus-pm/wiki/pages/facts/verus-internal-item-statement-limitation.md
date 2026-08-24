---
id: verus-internal-item-statement-limitation
type: fact
status: stable
title: Verus rejects static and extern items inside functions
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

Verus `0.2026.08.23.fbbbbcf` rejects a `static` item or `extern` block declared
inside a function body during HIR-to-VIR lowering:

```text
The verifier does not yet support the following Rust feature:
internal item statements
```

The limitation concerns item placement. The same `static` or `extern` declaration
at module scope is accepted, while a function-local `const` and nested `fn` are
also accepted. Ordinary Rust accepts all four forms.

Hoisting an inner foreign-symbol declaration to module scope preserves its
linkage and sole symbol identity. Hoisting an inner immutable `static` preserves
its single-instance, fixed-address, `'static`-lifetime semantics; only the name's
lexical scope broadens. Module visibility should match the inner item's effective
visibility.

An exhaustive brace-aware inventory of the 66 Nanvix PM files found two affected
declarations, both in `src/kernel/src/pm/process/manager/mod.rs`:
`forge_user_context` contained an inner `unsafe extern "C"` block, and
`write_nul_terminated_to_user` contained `static NUL: u8 = 0`. Hoisting those
items removed the family from the full layered probe (2 findings to 0). The next
already-present limitations in those declarations then became visible: a
`debug_assert!` panic lowering and a raw-pointer dereference. The fresh run
reached a three-round fixed point with an empty attribution-gap set and restored
all 66 probed files.

The minimized bad/good/legal-Rust cases, structured run-8 result, family delta,
and restoration evidence are under
`/home/ruize/argus-pm-artifacts-20260824/`; the durable campaign record is
`research/GROUND_TRUTH.md`.
