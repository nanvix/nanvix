# Polish Report: hal-frame-address

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks --all` reports 2 inline proof blocks, both already 1-line
    lemma calls (no block exceeds the 5-line threshold), so none required
    extraction:
    - `from_frame_number` (frame.rs:135): `proof! { lemma_frame_base_aligned(frame_number); }`
      → already delegates to `lemma_frame_base_aligned` in frame.proof.rs.
    - `into_frame_number` (frame.rs:152): `proof! { lemma_aligned_div_mul(self@); }`
      → already delegates to `lemma_aligned_div_mul` in frame.proof.rs.
- Blocks kept inline: 2 (each a single 1-line lemma call, ≤ 5 lines)
  - `from_frame_number` (frame.rs:135)
  - `into_frame_number` (frame.rs:152)

## Minimization
- Redundant assertions removed: 2
  - `assert(spec_page_size() == 4096);` in `lemma_frame_base_aligned` (frame.proof.rs)
  - `assert(spec_page_size() == 4096);` in `lemma_aligned_div_mul` (frame.proof.rs)
  - Both were hint asserts whose only role was to expose `spec_page_size() > 0`;
    Verus discharges `lemma_mod_multiples_basic` / `lemma_fundamental_div_mod`
    without them because `spec_page_size()` is the transparent `arch::mem::PAGE_SIZE`.
- Redundant lemmas/hints removed: 1 dead import + 2 stale comments
  - Removed unused `spec_max_frame_number` import from frame.spec.rs (no reference
    in any `requires`/`ensures`/`invariant`/lemma in the module).
  - Removed the 2 now-stale `// spec_page_size() is the transparent constant ...`
    comments that documented the deleted asserts.
- Redundant lemmas removed: 0
  - Both `lemma_frame_base_aligned` and `lemma_aligned_div_mul` prove distinct
    properties and are each load-bearing for an in-scope exec function; kept.

## Verification & Build
- `make verify-kernel MODULE=hal::mem::types::address::frame`: 5 verified, 0 errors.
- No admits in this module; only sanctioned `external_body` on `from_raw_value`
  (TCB-listed in `verus-ai-logs/tcb-allowed.md`).
- `./z build -- all-kernel`: Build complete.
- No spec drift: no `requires`/`ensures`/`invariant`/`view`/`inv` were modified;
  only redundant proof artifacts and an unused import were removed.
