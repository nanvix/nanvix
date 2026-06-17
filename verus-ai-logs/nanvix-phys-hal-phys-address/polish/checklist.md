# Polish Report: hal-phys-address

Scope: `src/kernel/src/hal/mem/types/address/phys.rs` and its `phys.spec.rs` /
`phys.proof.rs`. Verified with `make verify-kernel MODULE=hal::mem::types::address::phys`.
Final status: **6 verified, 0 errors, CLEAN** (no assume / admit / external_body in
any phys file). Standard build (`make all-kernel`) passes.

## Proof Extraction

- Blocks extracted: **0** — no inline `proof { ... }` block exceeds 5 lines, and the
  two existing blocks already delegate to named lemmas in `phys.proof.rs`
  (`lemma_from_number_no_overflow`, `lemma_frame_index`). Nothing to extract.
- Blocks kept inline: **2** (each ≤ 5 lines, only lemma calls):
  - `from_number` (phys.rs:151–153, 3 lines) → `lemma_from_number_no_overflow(frame)`.
  - `into_frame_number` (phys.rs:170–173, 4 lines) → call-site `lemma2_to64()` +
    `lemma_frame_index(self, raw_addr, FRAME_SHIFT, frame_number)`.

## Minimization

- Redundant assertions removed: **7** (all in `phys.proof.rs`; re-verified clean):
  1. `lemma_from_number_no_overflow`: `assert(raw <= m / s);`
     — derivable from `FrameNumber::spec_max() <= m/s` + the `requires`.
  2. `lemma_frame_index`: `assert(pow2(shift as nat) == 4096);`
     — derivable from `spec_page_size() == 4096` + `requires spec_page_size() == pow2(shift)`.
  3. `lemma_frame_index`: `assert(shift == 12);`
     — the two contradiction `if` blocks already exclude `shift < 12` and `shift > 12`.
  4. `lemma_frame_index`: `assert(frame_number as int == spec_frame_number(addr@));`
     — restates ensures clause 1.
  5. `lemma_frame_index`: `assert(s == 4096);`
     — redundant with `spec_page_size() == 4096` and the `let s = spec_page_size()` binding.
  6. `lemma_frame_index`: `assert(raw_addr as int <= m);`
     — automatic: `raw_addr` is `usize`, and `m == usize::MAX`.
  7. `lemma_frame_index`: `assert(frame_number as int <= spec_max_frame_number());`
     — restates ensures clause 2.

- Redundant lemmas/hints removed: **0**.
  - Tested removing the call-site `lemma2_to64()` in `into_frame_number`; verification
    failed (it discharges the `spec_page_size() == pow2(FRAME_SHIFT)` precondition of
    `lemma_frame_index`), so it was reverted.
  - All remaining proof-helper calls are load-bearing: `lemma_mod_bound`,
    `lemma_fundamental_div_mod`, `lemma_mul_inequality`, `lemma_pow2_strictly_increases`,
    `lemma_usize_shr_is_div`, `unsigned_int_max_values`, `lemma_div_is_ordered`.
  - The bridge asserts `s == FRAME_SIZE` / `m == MAX_ADDRESS` / `spec_max() <= m/s` in
    `lemma_from_number_no_overflow` are load-bearing (`spec_max()` is defined over
    `FRAME_SIZE`/`MAX_ADDRESS` in the `arch` crate), so they were kept.

- Dead spec functions removed: **0** — `spec_frame_raw_value`, `spec_max_frame_number`,
  `spec_frame_number`, `spec_from_number`, and `inv` are all `pub` and consumed by
  `frame.spec.rs` / `frame.proof.rs` (module API); none are dead.

## Result

- `phys.rs`: unchanged. `phys.spec.rs`: unchanged. `phys.proof.rs`: 7 assertions removed.
- No spec drift: no `requires` / `ensures` / `invariant` or lemma signature was changed.
- `make verify-kernel MODULE=hal::mem::types::address::phys` → 6 verified, 0 errors, CLEAN.
- `make all-kernel` → build succeeds.
