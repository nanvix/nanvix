## Turn 1: Zero remaining admit() (checklist item 2)

### Progress
- Done (verified PASS):
  - **Item 8 — Exec rewrites minimal**: PASS. No `// VERUS REWRITE` comments exist in
    `src/kernel/src/mm/virt/` — nothing claimed, nothing to audit.
  - **Item 9 — Cross-module regression**: PASS. `make verify-kernel` ran `<all>` modules and
    exited 0 (cached, no recompilation). No verified module regressed to an error.
- Current: **Item 2 — Zero remaining admit()** → **FAIL** (3 admits). This is the headline
  blocker and the entire purpose of the proving phase. Addressed in detail below.
- Remaining / secondary findings recorded but NOT yet resolved (will be re-driven on later
  turns, after the admits are discharged, since removing the admits changes what the bodies
  must prove):
  - Item 1 — No specs weakened: NOT YET CONFIRMABLE (see note).
  - Item 3 — external_body audit: 1 occurrence to confirm (`ExPageTableBss`).
  - Item 4 — assume/assume_specification audit: 2 occurrences to confirm.
  - Item 5 — No cfg-gated exec code: **FAIL** (5 occurrences).
  - Item 6 — Cheating audit counts: reported below.
  - Item 7 — Verus-limitation reproducers: none present.
  - Item 10 — Verification 0 errors/0 warnings: status is `CHEATING_DETECTED`, not clean.

### Verification

Commands run:
- `make verify-kernel` → Exit 0, but: `cheating: assume=0 external_body=12 admit=31
  trusted=0 cfg_gate=15`, `status: CHEATING_DETECTED`.
- Read `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`, filtered to this module.
- Read the three exec bodies and their `#[verus_spec]` contracts in
  `src/kernel/src/mm/virt/identity_map.rs`.
- `grep -n "admit"` on `identity_map.rs`.

**Module-scoped cheating audit (item 6):**
| Pattern | Count | Locations (this module) |
|---|---|---|
| `admit` | **3** | `identity_map.rs:534` (`ensure_pt`), `identity_map.rs:632` (`ensure_pte`), `identity_map.rs:719` (`identity_map_page`) |
| `external_body` | 1 | `identity_map.spec.rs:142` — `ExPageTableBss` (`external_type_specification` + `external_body`) |
| `assume_specification` | 2 | `identity_map.spec.rs:179` (`<[T]>::as_ptr`), `identity_map.spec.rs:183` (`FixedSizeBumpAllocator::<N,A,S>::new`) |
| cfg-gated exec | 5 | `identity_map.rs:537,553,565,635,648` — `#[cfg(not(verus_keep_ghost))]` over `error!(...)` exec statements |

The crate-wide totals (`admit=31`, `external_body=12`, `cfg_gate=15`) include sibling
modules (`mm/phys/*`, `hal/*`) that are out of scope for THIS target; the per-module rows
above are the ones I am holding you to.

**Item 2 detail — the three admits.** Each in-scope exec function opens its body with
`proof! { admit(); }`, which discharges its ENTIRE postcondition vacuously. The contracts
these admits are hiding are non-trivial and are the whole deliverable of this module:

1. `ensure_pt` (`identity_map.rs:533`, admit at `:534`)
   - Must prove on `Ok(pt_paddr)`: `identity_map_view().inv()` **and**
     `spec_is_page_aligned(pt_paddr as int)`; on `Err`: `identity_map_view().inv()`.

2. `ensure_pte` (`identity_map.rs:627`, admit at `:632`)
   - Must prove `identity_map_view().inv()` always, and on `Ok`:
     `identity_map_view().mapped.contains(spec_page_base(phys_addr as int))`; on `Err`:
     `!identity_map_view().mapped.contains(spec_page_base(phys_addr as int))`.

3. `identity_map_page` (`identity_map.rs:718`, admit at `:719`)
   - Must prove `identity_map_view().inv()` always, and on `Ok`:
     `identity_map_view().accessible(phys_addr@)`; on `Err`:
     `!identity_map_view().accessible(phys_addr@)`.

These postconditions reference the **uninterpreted** `identity_map_view()` (spec.rs:36) and
the `v -> v'` transition that the spec/TCB notes say is "deferred to the proving-phase
page-table permission token". The proving phase is exactly where that deferral must be paid
off. The proof-side lemmas already exist and are admit-free
(`identity_map.proof.rs`: `lemma_install_page_maps`, `lemma_install_page_monotone`,
`lemma_install_page_preserves_inv`, `lemma_map_page_accessible`,
`lemma_map_page_preserves_inv`) — they give you the View-level facts
(`spec_install_page`/`spec_map_page` ⇒ `mapped.contains` / `accessible` / `inv`). What is
missing is the connection from the exec page-table writes (`pd.write`, `pt.write`,
`paging::invlpg`) to the abstract `identity_map_view()` transition, i.e. the ghost token /
attachment that the in-scope bodies must thread so those lemmas can fire.

**Item 1 note (not yet confirmable):** I will not mark "no specs weakened" PASS while the
bodies are admit-gated. With `admit()` at the top of every body, Verus never checks that the
exec code actually establishes these postconditions, so the contracts are currently
unfalsifiable. A weakened spec and a correct-but-unproven spec are indistinguishable under
`admit`. Re-audit deferred until item 2 is fixed.

### Fix Request

**Primary (item 2) — remove all three `admit()`s and prove the real contracts.** Do them
one function at a time; do not batch.

1. **`ensure_pte` first** (the V==P leaf, `identity_map.rs:627`). Delete
   `proof! { admit(); }` at line 632. Thread the page-table permission/ghost token so the
   `pt.write(pte_idx, new_pte)` step is reflected as `spec_install_page` on
   `identity_map_view()`, then invoke the existing `lemma_install_page_maps` /
   `lemma_install_page_preserves_inv` to discharge `mapped.contains(spec_page_base(...))`
   and `inv()`. For the idempotent `is_present()` fast-path and the `Err` arms, prove the
   page's membership/non-membership matches the stated post.

2. **`ensure_pt`** (`identity_map.rs:533`). Delete the admit at line 534. Prove
   `inv()` is preserved on both arms and `spec_is_page_aligned(pt_paddr as int)` on `Ok`
   (the BSS slot base is page-aligned — connect `slot.as_ptr() as usize` to the page-size
   multiple fact). Installing an empty PT adds no page, so `mapped` is unchanged.

3. **`identity_map_page`** (`identity_map.rs:718`). Delete the admit at line 719. Compose
   the two proven callees with `lemma_map_page_accessible` /
   `lemma_map_page_preserves_inv` to discharge `accessible(phys_addr@)` / `inv()`, including
   the pre-init `pd_paddr == 0` no-op arm (boot tables ⇒ `!initialized` ⇒ `accessible`).

**Verify after each:** run `make verify-kernel` and confirm the module's admit count drops.
The acceptance bar is the module row reading `admit=0` for
`mm/virt/identity_map.*` in `verus-ai-logs/verify-kernel/verus-logs/cheating-detail.txt`,
with `make verify-kernel` still exit 0.

**Do NOT** replace the admit with an equally-strong-looking comment, a `#[verifier::external_body]`,
or an `assume`/`assume_specification` that re-hides the same obligation — that is not a fix.
If you believe a specific sub-step is a genuine Verus limitation, produce an **isolated**
reproducer for that single construct (item 7) before relying on any trust boundary, and the
trust boundary must be a real `#[verus_spec]` contract, not `admit`.

**Secondary items to fix this phase (will be re-driven after the admits land):**
- **Item 5 (cfg-gated exec — FAIL):** the 5 `#[cfg(not(verus_keep_ghost))]` guards over
  `error!(...)` at `identity_map.rs:537,553,565,635,648` make the body Verus verifies differ
  from the body that ships. Remove the cfg gating so the `error!` calls are present in both
  configurations (or move them out of the verified exec path) — exec code must not be
  cfg-gated away from the verifier.
- **Item 4:** justify or remove the two `assume_specification`s. `<[T]>::as_ptr` is a std
  external-bottom boundary (acceptable). `FixedSizeBumpAllocator::<N,A,S>::new`
  (spec.rs:183) is a not-yet-verified workspace dependency — per this module's own
  "placeholders are removed when the dependency module is verified" policy, confirm whether
  `bump_allocator` is now verified and, if so, drop the placeholder in favor of its real
  contract.
- **Item 3:** confirm `ExPageTableBss` (`external_type_specification` + `external_body`,
  spec.rs:141-143) is a pure opaque-type registration (no exec body trusted). If the
  reviewer/TCB policy requires it, add an explicit entry to `verus-ai-logs/tcb-allowed.md`;
  otherwise show it carries no trusted exec contract.

STOP file NOT created — items 1, 2, 3, 4, 5, 6, 7, 10 are not yet PASS/FIXED.
