# Polish Report: phys-frame

Module: `src/kernel/src/mm/phys/frame.rs`
Verification: `make verify-kernel MODULE=mm::phys` → **80 verified, 0 errors** (exit 0).
Build: `./z build -- check-kernel` → success.
No new `admit`/`external_body` introduced (counts unchanged from baseline:
`external_body=23, admit=4`, all pre-existing in other `mm::phys` modules).
Protected specs and spec/view definitions: unchanged (`frame.spec.rs` byte-identical;
`frame.proof.rs` changes are purely additive — no existing definition modified).

Result: all 49 `proof { }` blocks are now ≤ 5 lines (down from 37 over the threshold).
`frame.rs` shrank 1697 → 1286 lines; proof reasoning moved into 13 named lemmas in
`frame.proof.rs` (608 → 1168 lines).

## Proof Extraction

- Blocks extracted: 17 (into 13 named lemmas in `frame.proof.rs`)

  | Old location (function / block)            | Orig. lines | New lemma                              |
  |--------------------------------------------|-------------|----------------------------------------|
  | `alloc` prologue (view + inv-facts)        | 13          | `lemma_capture_inv_facts`              |
  | `alloc` bitmap-full Err arm                | 15          | `lemma_alloc_full_no_free`             |
  | `alloc` reserve Ok arm                     | 15          | `lemma_post_reserve_one_by_index`      |
  | `alloc_contiguous` prologue                | 11          | `lemma_capture_inv_facts`              |
  | `alloc_contiguous` inv re-establishment    | 32          | `lemma_reestablish_inv_range`          |
  | `alloc_contiguous` reserve-range Ok arm    | 72          | `lemma_alloc_contiguous_post`          |
  | `alloc_range` prologue (view + geometry)   | 22          | `lemma_capture_inv_facts` + `lemma_alloc_range_geometry` |
  | `alloc_range` scan: uncovered index        | 21          | `lemma_range_uncovered_not_all_free`   |
  | `alloc_range` scan: allocated index        | 23          | `lemma_range_allocated_not_all_free`   |
  | `alloc_range` epilogue (inv + view rebuild)| 89          | `lemma_alloc_range_post`               |
  | `free` allocated-frame check               | 14          | `lemma_frame_allocated`                |
  | `free` last-reference release Ok arm       | 11          | `lemma_post_release_one`               |
  | `free` shared-frame else arm               | 14          | `lemma_post_update_slot`               |
  | `share` allocated-frame check              | 13          | `lemma_frame_allocated`                |
  | `share` increment else arm                 | 14          | `lemma_post_update_slot`               |
  | `book` reserve Ok arm                       | 16          | `lemma_post_reserve_one`               |
  | `refcount` allocated-frame final check     | 11          | `lemma_frame_allocated`                |

  13 new lemmas: `lemma_capture_inv_facts`, `lemma_frame_allocated`,
  `lemma_post_update_slot`, `lemma_post_release_one`, `lemma_post_reserve_one`,
  `lemma_post_reserve_one_by_index`, `lemma_alloc_full_no_free`,
  `lemma_reestablish_inv_range`, `lemma_alloc_contiguous_post`,
  `lemma_alloc_range_geometry`, `lemma_range_uncovered_not_all_free`,
  `lemma_range_allocated_not_all_free`, `lemma_alloc_range_post`.
  Several are shared across call sites (e.g. `lemma_capture_inv_facts` ×3,
  `lemma_frame_allocated` ×3, `lemma_post_update_slot` ×2), removing duplication.

- Blocks kept inline: 32 (each ≤ 5 lines — single lemma call, a ghost-freezing
  assert, a loop-invariant instantiation, or an unreachable-arm `assert(false)`;
  all tightly coupled to local exec/loop context where extraction would only add
  indirection).

## Minimization

- Redundant assertions removed: ~271 inline proof/assert/hint lines dropped from
  `frame.rs` (411 net exec lines removed, 23 lemma-call lines added). Examples:
  - Six near-identical 12-line `free`/`share`/`book`/`refcount`/`is_covered`
    prologues trimmed to 2–4 lines by deleting `assert(addr >= 0)`,
    `assert(addr % spec_page_size() == 0)`, `assert(fnx == addr / spec_page_size())`,
    and `assert(spec_refcount_seq(self) == pre_rc)` — all auto-derivable from
    `frame.inv()` / the `into_frame_number` contract.
  - Out-of-bounds / refcount-zero / scan blocks reduced to the single load-bearing
    `assert(!self.bitmap@.set_bits.contains(..))` / coverage assert.
  - `alloc_range` geometry prologue: 8 restated `region.inv()`/division asserts
    dropped (auto-derivable), keeping only `ps == 4096`.
  - Verbose two-line explanatory comments moved outside `proof! { }` blocks so the
    proof bodies fit ≤ 5 lines without losing documentation.

- Redundant lemmas/hints removed: 0 lemmas deleted (no existing lemma duplicated
  another's `requires`+`ensures`). Redundant inline proof *hints* folded into the
  new lemmas or dropped from exec code, including `lemma_view_of` (duplicate calls),
  `lemma_internal_inv_facts`, `lemma_size_div_pos`, `lemma_aligned_div_sum`,
  `vstd::arithmetic::div_mod::lemma_div_pos_is_pos`,
  `vstd::arithmetic::div_mod::lemma_div_is_ordered`, and the per-call-site
  `lemma_reserve_one_v` / `lemma_release_one_v` / `lemma_update_refcount_v` /
  `lemma_reserve_range_v` / `lemma_view_of_all_free` invocations (now reached only
  through the extracted lemmas).
