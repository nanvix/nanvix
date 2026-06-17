# Polish Report: bump-allocator

Scope: `FixedSizeBumpAllocator::alloc_as`, `FixedSizeBumpAllocator::alloc`,
`align_up`, `as_mut_ptr`.
Verification after every change: `make verify-bump-allocator` → **6 verified, 0 errors**
(exit 0, `assume=0 admit=0`; `external_body=2` are the registered `alloc`/`alloc_as`).
Non-Verus build: `cargo build -p bump-allocator` → success.

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py lib.rs --all` reports **no inline `proof {}` blocks**.
    The in-scope exec functions `alloc`/`alloc_as` are `external_body` and
    `align_up`/`as_mut_ptr` carry no proof blocks, so there is nothing inline to
    extract. All proof content already lives in named lemmas in `lib.proof.rs`
    (`lemma_geometry`, `lemma_exhausted_boundary`, `lemma_alloc_transition`).
- Blocks kept inline: 0 (none exist)

## Minimization
- Redundant assertions removed: 2
  1. `assert(s == a * q);` in `lemma_geometry` (lib.proof.rs) — the fact is
     already available from `lemma_fundamental_div_mod` + `inv` and discharges the
     `requires s == a * q` of the nested `nonlinear_arith` block without it.
  2. `assert(v.spec_alloc() =~= (BumpView { allocated: ..., ..v }));` in
     `lemma_alloc_transition` (lib.proof.rs) — `BumpView` derives
     `#[verifier::ext_equal]`, so Verus proves the `==` ensures structurally;
     the lemma body is now empty.
- Redundant lemmas/hints removed: 1
  1. `vstd::arithmetic::mul::lemma_mul_nonnegative(i, s);` (lower-bound hint) in
     `lemma_geometry` part (b) — Verus derives `i * s >= 0` in context without it.
- Stale comment cleaned: removed the inaccurate header note claiming lemma bodies
  are `admit()` placeholders "during the specification phase" (proofs are fully
  discharged).

### Hints confirmed load-bearing (kept; removal re-broke verification)
- `lemma_mul_is_distributive_add_other_way` + `lemma_mul_inequality` (part b upper bound).
- The `i < j` / `else` `lemma_mul_strict_inequality` pair (part c distinctness).
- `nonlinear_arith` `i * s == a * (i * q)` + `lemma_mod_multiples_vanish` (part a).

### Not removed (per proof-minimization rules)
- `lemma_geometry`, `lemma_exhausted_boundary` are uncalled but prove distinct,
  `pub` caller-facing properties — kept (Verus verifies them regardless of callers).
- All spec functions are `pub` and referenced — none dead.
