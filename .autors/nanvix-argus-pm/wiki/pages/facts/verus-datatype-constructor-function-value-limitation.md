---
id: verus-datatype-constructor-function-value-limitation
type: fact
status: stable
title: Verus rejects datatype constructors used as function values
sources: []
created_at: 2026-08-24
last_reviewed_at: 2026-08-24
---

The latest Verus main release tested on 2026-08-24 rejects a datatype constructor
used as a first-class function value during frontend lowering:

```text
The verifier does not yet support the following Rust feature:
using a datatype constructor as a function value
```

The limitation covers tuple-variant constructors, tuple-struct constructors,
`Self` for tuple structs, and standard constructors such as `Result::Err`.
Ordinary Rust accepts these values as `FnOnce` arguments to combinators such as
`Option::map` and `Result::map_err`; Verus accepts and verifies the equivalent
explicit closure `|x| Constructor(x)`. The failure therefore precedes proof
obligations and is not caused by a missing contract, invariant, lemma, caller
fact, type error, or SMT difficulty.

For a one-argument constructor, the explicit closure preserves the combinator
input, output, evaluation branch and order, moves and borrows, error path, state
transitions, side effects, unsafe behavior, and concurrency semantics. Nanvix
must locally allow Clippy's `redundant_closure` lint because Clippy otherwise
recommends the Verus-rejected form.

In `src/kernel/src/pm/**`, exhaustive source inspection found 11 instances across
six files. The prior probe reported only six affected declarations containing
ten of those sites; an eleventh use in `join_thread` was hidden behind an earlier
cast diagnostic in the same function. Rewriting all 11 uses removed the family
from the fresh 66-file layered probe (six reported declarations to zero) without
changing any other diagnostic family. The probe reached a fixed point in three
rounds, the ordinary kernel check passed, and a direct comparison confirmed
restoration of all 66 instrumented source files.

The durable run record is in `research/GROUND_TRUTH.md`. The minimized
reproducers, run-6 structured result, family delta, and restoration evidence are
under `/home/ruize/argus-pm-artifacts-20260824/`.
