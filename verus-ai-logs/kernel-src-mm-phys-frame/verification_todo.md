# Verification TODOs: frame module

All items previously listed here (assume_specification[init] and the 4
singleton wrappers) have been reclassified as external-bottom trust boundaries
and moved to `trust.md` (entries 9-13). They are genuine Verus limitations
involving `static mut`, `MaybeUninit`, and `AtomicBool` that cannot be
resolved without Verus adding support for these constructs.

The inner methods (`Inner::alloc`, `Inner::free`, `Inner::book`,
`Inner::alloc_range`) are fully body-verified with rich specifications.
The proof gap exists only at the singleton accessor boundary.

**Resolution path:** When Verus adds support for `static mut` or provides a
verified singleton abstraction, these items can be resolved by:
1. Writing `assume_specification` for `AtomicBool::load/store` and
   `MaybeUninit::assume_init_mut/write`
2. Verifying `instance()` and `init()` with those specs
3. Removing `external_body` from the pub wrappers and propagating
   Inner method specs through the singleton accessor

**Current status: No open verification TODOs.**
