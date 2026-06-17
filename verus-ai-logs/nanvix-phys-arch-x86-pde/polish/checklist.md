# Polish Report: arch-x86-pde

## Proof Extraction
- Blocks extracted: 0
- Blocks kept inline: 2 (each a single-line lemma call, ≤ 5 lines)
  - `pde.rs:318` in `PageDirectoryEntry::new` — `proof! { use_type_invariant(frame); }` (1 line)
  - `pde.rs:431` in `PageDirectoryEntry::frame_address` — `proof! { lemma_frame_address(raw); }` (1 line)

`check_proof_blocks.py --all` reports 2 proof blocks, 0 over 5 lines. The only
non-trivial proof was already factored into the named lemma `lemma_frame_address`
in `pde.proof.rs`, so no extraction was required.

## Minimization
- Redundant assertions removed: 3 (all in `lemma_frame_address`, `pde.proof.rs`)
  - `assert(pow2(FRAME_SHIFT as nat) == FRAME_SIZE)` — provided by the
    `lemma2_to64()` broadcast already called above it.
  - `assert(FrameNumber::spec_max() <= m / s)` — intermediate, derivable from
    the `open` definition of `spec_max`.
  - `assert(raw as int <= m / s)` — intermediate, derivable from the `requires`
    bound plus the line above.
- Redundant lemmas/hints removed: 1
  - Dropped the now-unused `pow2` import from
    `use vstd::arithmetic::power2::{pow2, lemma2_to64};` → `lemma2_to64;`
    (its only use was the removed assertion).

### Kept (load-bearing — verified by removal test, reverted on failure)
- `lemma2_to64()` — removing it fails verification (supplies `pow2(12)==4096`).
- `lemma_mod_bound`, `lemma_fundamental_div_mod`, `lemma_mul_inequality`,
  and the `by (nonlinear_arith)` block — each discharges an explicit `requires`
  of the no-overflow product bound `raw * FRAME_SIZE <= MAX_ADDRESS`.
- `lemma_usize_shl_is_mul`, `lemma_mod_multiples_basic` — discharge the two
  `ensures` (shift==mul, FRAME_SIZE-alignment).

### Not removed
- All spec functions in `pde.spec.rs` are `pub` (part of the module API for
  dependents such as `pte.spec.rs`); per the minimization skill, `pub` spec
  functions are kept. No non-public/dead spec functions exist.
- The `VERUS REWRITE` comment in `frame_address` is substantive rewrite
  documentation (with reproducer reference), not an obsolete proof hint — kept.

## Verification
- `make verify-arch`: 47 verified, 0 errors (exit 0).
- Cheating check: `assume=0 admit=0` in scope; pre-existing `external_body=3`,
  `cfg_gate=2` live in other arch files (none in `pde*.rs`) and are unchanged.
- `pde.spec.rs` and `pde.rs` byte-for-byte unchanged → no spec drift / weakening.
