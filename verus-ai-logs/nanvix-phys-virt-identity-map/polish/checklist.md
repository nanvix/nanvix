# Polish Report: virt-identity-map

## Scope

In-scope functions: `identity_map_page`, `ensure_pt`, `ensure_pte`
(all `#[verus_verify(external_body)]`, TCB-listed in `verus-ai-logs/tcb-allowed.md`).

## Proof Extraction

- Blocks extracted: 0
  - `check_proof_blocks.py --all` reports **"No proof blocks found."** All three
    in-scope functions are `external_body`, so their bodies are not translated by
    Verus and contain no inline `proof { ... }` blocks. The only proof code lives in
    `identity_map.proof.rs` as already-named transition lemmas. Nothing to extract.
- Blocks kept inline: 0

## Minimization

- Redundant assertions removed: 9
  All `assert(...)` hints across the five transition lemmas were provable
  automatically by Verus (open-spec unfolding + ambient `Set::insert` axioms):
  - `lemma_install_page_maps`: 1 (`=~=` insert assert)
  - `lemma_install_page_monotone`: 2 (`=~=` assert + inner `insert.contains(x)`)
  - `lemma_install_page_preserves_inv`: 3 (`=~=` assert + `insert.contains(p)` + `mapped.contains(p)`)
  - `lemma_map_page_accessible`: 3 (`==`, `=~=`, and `contains(page)` asserts)
  - `lemma_map_page_preserves_inv`: 0

- Redundant lemmas/hints removed: 6
  - 2 × `assert forall ... by { ... }` proof-hint blocks (`monotone`, `preserves_inv`)
  - 2 × `if v.initialized { ... }` case-split scaffolds (`map_page_accessible`, `map_page_preserves_inv`)
  - 1 × inter-lemma call `lemma_install_page_preserves_inv(v, page)` (`map_page_preserves_inv`)
  - 1 × `let v2 = v.spec_install_page(page)` binding (`preserves_inv`)
  - 0 whole lemmas removed: all five `pub` transition lemmas are distinct
    (distinct `requires`/`ensures`) and retained as module proof API.

## Spec-drift Check

- `identity_map.spec.rs`: **byte-identical** to pre-polish (no spec change).
- `identity_map.proof.rs`: only proof-body statements removed; every lemma
  signature, `requires`, and `ensures` preserved verbatim. No guarantee weakened.

## Verification Status

- `make verify-kernel MODULE=mm::virt::identity_map`
  → `6 verified, 0 errors` (exit 0).
- Module `mm::virt::identity_map`: assume=0, admit=0, trusted=0.
  (Global `admit=4` are all in the out-of-scope `mm/phys/manager` module.)
- 3 `external_body` flags are the pre-existing TCB-listed in-scope functions
  (not introduced by this pass).
