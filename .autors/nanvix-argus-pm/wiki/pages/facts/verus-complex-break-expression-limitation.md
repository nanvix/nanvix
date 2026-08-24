---
id: verus-complex-break-expression-limitation
type: fact
status: stable
title: Verus rejects value-carrying break expressions
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

The latest Verus main release tested on 2026-08-24 (`0.2026.08.23.fbbbbcf`,
source HEAD `fbbbbcf`) rejects a value-carrying `break EXPR` — a `break` that
carries a non-unit value out of a `loop` — during frontend lowering:

```text
The verifier does not yet support the following Rust feature:
complex break expressions
```

A plain `break;` (no value) is accepted; only the value-carrying form trips the
HIR→VIR lowering, so the failure precedes proof obligations and is not caused by
a missing contract, invariant, lemma, caller fact, type error, or SMT
difficulty. Ordinary Rust accepts `break EXPR` as the way a `loop` yields a
value.

Two behaviour-preserving rewrites are accepted and verify:

- In a **function-tail** loop (a `loop` that is the function's tail expression,
  so the loop's value is the function result), `break VALUE` is exactly
  `return VALUE`.
- In a **value-position** loop (a `loop` used as the initializer of a `let` or
  other expression), declare a variable before the loop, assign the carried
  value to it immediately before a plain `break;`, and let the surrounding
  block's tail read that variable. The variable is definitely-assigned because
  the loop's only non-diverging exit is that single plain break.

In `src/kernel/src/pm/**`, an exhaustive `\bbreak\b` scan of all 66 PM files
found exactly four value-carrying break sites across three files / three
declarations (all other `break` uses are plain `break;` / `break,` or appear in
comments): `sync/mutex.rs::lock` (`break Ok(guard)`, tail loop),
`process/manager/unsafe.rs::join_thread` (two sites: `break Ok(status)` and
`break Err(SleepError::Generic(error))`, tail loop), and
`process/manager/signal.rs::try_deliver_signal` (`break (signum, entry, mask,
flags)`, value-position `let`-initializer loop). The run-6 probe reported three
complex-break declarations; `join_thread` contributes two break sites in one
declaration, so the expected family delta is 3 → 0 despite four source edits.

The fresh 66-file layered run-7 confirms that rewrite removes the family:
complex-break declarations fell from 3 to 0, every other diagnostic family was
unchanged, and the residual total fell from 32 to 29 (26 `LIMITATION`, 3
`INCONCLUSIVE`). The probe reached a three-round fixed point with a fully
enumerated terminal frontier, no attribution gaps, and byte-identical
restoration of all 66 PM files.

The minimized reproducers (`complex_break_bad.rs` → complex-break errors;
`complex_break_good.rs` → verified, 0 errors;
`complex_break_legal_rust_check.rs` → legal Rust), structured run-7 result,
family delta, and restoration evidence are under
`/home/ruize/argus-pm-artifacts-20260824/`.
