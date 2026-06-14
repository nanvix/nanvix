# Verification TODOs: phys-upool

The `mm::phys::upool` module verifies cleanly
(`make verify-kernel MODULE=mm::phys` → 42 verified, 0 errors, exit 0).

There are **no** `admit()`, `assume()`, `assume_specification`,
`#[verifier::exec_allows_no_decreases_clause]` (R20p), or `limitation_assume`
(R20c) anywhere in `upool.rs`, `upool.spec.rs`, or `upool.proof.rs`. The proof
file is empty (`verus! { }`). All eight target `UserFrame`/`Upool` functions
discharge their contracts; the only remaining `external_body` are the two below.

## Remaining `external_body` (genuinely irreducible in the specification phase)

These two are NOT proof gaps that further effort can close in this phase; they
are mathematically un-dischargeable in-body given the project's
frozen-`phys_view()` specification convention (cross-call state transitions are
deferred to a proving-phase ghost token over the singleton allocator). Both are
authorized in `verus-ai-logs/tcb-allowed.md`. Evidence was produced by removing
the attribute and re-running Verus.

### 1. `Upool::new` — `external_body`
- **Contract:** `ensures result@.wf()`.
- **Why irreducible:** `View for Upool` is `uninterp spec fn view(&self) ->
  FrameAllocView;` (the pool carries no spec-readable state — its real state is
  the global frame allocator). With an uninterpreted view, `result@` is an
  unknown `FrameAllocView`, so `result@.wf()` cannot be established from the
  body `Self { _private: () }`.
- **Verus evidence (attribute removed):**
  ```
  error: postcondition not satisfied
  243 |             result@.wf(),
      |             ^^^^^^^^^^^^ failed this postcondition
  ```
- **Blocker pattern:** an in-body proof would require a concrete `View for
  Upool`, but no concrete pure-spec view satisfies BOTH `new` (needs
  `wf()` unconditionally) AND `alloc` (needs a real state transition) — see
  below. The view must stay `uninterp`, hence `new` stays `external_body`.

### 2. `Upool::alloc` — `external_body`
- **Contract (Ok arm):** `old(self)@.free_frames.contains(uf@)` and
  `final(self)@ == old(self)@.alloc_one(uf@)`.
- **Why irreducible:** the body's only state-changing call is `frame::alloc()`,
  whose own contract speaks **only** of the parameter-free, frozen global
  `phys_view().frames` (the `v -> v'` transition is deferred to the proving-phase
  ghost token; `phys_view()` is an uninterpreted constant within this phase).
  `frame::alloc` says nothing about `self`, and `self` (`Upool { _private: () }`)
  is structurally unchanged across the call, so no in-body reasoning can derive
  the `self@` transition `final(self)@ == old(self)@.alloc_one(uf@)`. A constant
  view would make the Ok arm provably false (a frame `frame::alloc` reports as
  allocated cannot also be in `old(self)@.free_frames` under `wf()` disjointness),
  confirming no pure-spec view works.
- **Verus evidence (attribute removed):**
  ```
  error: postcondition not satisfied   (final(self)@ == old(self)@.alloc_one(uf@))
  error: postcondition not satisfied   (old(self)@.free_frames.contains(uf@))
  ```
- **Eliminated when:** the `frame` free-function layer's transitions are realized
  by the §8 proving-phase ghost token (separate phase `nanvix-phys-phys-frame`),
  at which point `frame::alloc` can expose a real `phys_view'` transition and
  this wrapper can be proven in-body. Out of scope for `phys-upool` (would
  require touching `frame.rs`, an unlisted file).

## Eliminated this phase
- `Upool` (struct) `external_body` was **removed** (now plain `#[verus_verify]`).
  The struct `{ _private: () }` is trivially modeled by Verus; the `external_body`
  trust boundary was unnecessary. Verified: 42 verified, 0 errors. This reduced
  the module `external_body` count from 3 to 2 (global 18 → 17).

## Out of scope (other phases — hard rule "do not touch unlisted functions")
The remaining module-wide `admit` (12) and `external_body` (the frame/manager/
mod/kframe entries) reported by `make verify-kernel MODULE=mm::phys` live in
`frame.rs`, `manager.rs`/`manager.proof.rs`, `mod.rs`, and `kframe.rs`. They
belong to the separate phases `nanvix-phys-phys-frame`,
`nanvix-phys-phys-manager`, `nanvix-phys-phys-mod`, and `nanvix-phys-kframe`,
and are not target functions of `phys-upool`. They are unchanged by this phase.
