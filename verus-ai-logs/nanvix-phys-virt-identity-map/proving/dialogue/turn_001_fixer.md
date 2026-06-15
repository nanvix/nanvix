## Response to Turn 1

### What I Changed

**`src/kernel/src/mm/virt/identity_map.proof.rs` — all 5 lemmas proven (real bodies, no `admit()`).**
In-scope `admit()` count dropped **8 → 3**. These are genuine `Set<int>`/arithmetic proofs as the reviewer expected:

| Lemma | Line | Proof technique |
|---|---|---|
| `lemma_install_page_maps` | 14 | `=~=` set-extensionality: `spec_install_page(page).mapped =~= mapped.insert(page)`, then `insert().contains(page)`. |
| `lemma_install_page_monotone` | 23 | extensionality + `assert forall x in mapped` ⇒ `mapped.insert(page).contains(x)`. |
| `lemma_install_page_preserves_inv` | 36 | extensionality + `forall p in insert(page)` case-split (`p == page` aligned by hyp; `p != page` ⇒ `mapped.contains(p)` ⇒ aligned by `v.inv()`). |
| `lemma_map_page_accessible` | 56 | case-split on `v.initialized`; initialized arm reduces `spec_map_page` to `spec_install_page` and reuses the `insert().contains` fact. |
| `lemma_map_page_preserves_inv` | 68 | case-split on `v.initialized`; delegates to `lemma_install_page_preserves_inv`. |

No spec was weakened, no `external_body`/`assume`/`admit` added, no exec signature changed.

### Verification

`make verify-kernel MODULE=mm::virt::identity_map`:
```
verification: 9 verified, 0 errors (exit 0)
cheating: assume=0 external_body=20 admit=7 trusted=0 no_decreases=0 cfg_gate=19
In-scope admits (this module):
  - identity_map.rs:533 ensure_pt: admit
  - identity_map.rs:627 ensure_pte: admit
  - identity_map.rs:718 identity_map_page: admit
```
Module: **9 verified, 0 errors.** In-scope `admit` = **3** (the three exec functions only; the 5 lemmas are now fully proven).

Normal build (Verus-erased), kernel crate:
```
cargo build --locked -p kernel --target build/targets/x86-kernel.json \
  -Zbuild-std=core,alloc,compiler_builtins \
  -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec
    Finished `dev` profile [optimized + debuginfo] target(s) in 12.49s
```
**Build OK.**

### Result: FIXED (5 lemmas) / NEEDS_GUIDANCE (3 exec functions)

**FIXED:** All 5 lemmas the reviewer flagged as "plain Set/arithmetic facts" are now genuinely proven (admit 8→3).

**NEEDS_GUIDANCE:** The 3 exec functions cannot reach `admit=0` without an action currently forbidden by the task rules. I removed each `admit()` and attempted real proofs; here are the **exact** remaining errors and why each is a deferred-ghost-token / dependency-spec-minimality blocker, not a proof-skill gap.

#### 1. `ensure_pt` (identity_map.rs:533) — blocked by `bump_view(self).inv()`

I applied the cross-crate type-invariant workaround `pde_idx.into_raw()` (table.rs:85-93, `ensures result < PAGE_TABLE_LENGTH`) and a `by (compute)` alignment fast-path. With those, the `pd.read` index precondition **and** the present-path alignment both discharge. The **only** remaining error:
```
error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:555:9
556 | |  .alloc_as::<[PteWord; PAGE_TABLE_LENGTH]>()
350 |    requires bump_view(self).inv(),   (bump_allocator/src/lib.rs:350)
    |             --------------------- failed precondition
```
`bump_view` is a parameter-free `uninterp spec fn` (bump_allocator `lib.spec.rs`) with **no establishing lemma and no `type_invariant`** anywhere in the tree:
```
$ grep -rn "bump_view" src --include=*.rs | grep -iE "ensures|inv\(\)|broadcast|axiom"   → (none)
$ grep -rn "type_invariant" src/libs/bump_allocator/                                       → (none)
```
No caller can prove `inv()` for the concrete `PAGE_TABLE_ALLOCATOR` static — this is the bump allocator's own deferred ghost token. The alloc path is unavoidable in `ensure_pt`, so this single obligation makes the function unprovable in-body.

#### 2. `ensure_pte` (identity_map.rs:627) — blocked by `mapped.contains` postcondition

```
error: precondition not satisfied         --> identity_map.rs:637:40   (pt.read index — solvable via into_raw)
error: postcondition not satisfied        --> identity_map.rs:624:13
    |  Ok  => identity_map_view().mapped.contains(spec_page_base(phys_addr@))
    |  Err => !identity_map_view().mapped.contains(spec_page_base(phys_addr@))
```
`Table::write` (table.rs:246) is deliberately **contents-free** (`external_body`; a contents postcondition would be UNSOUND per its TCB note), and `identity_map_view()` is a parameter-free `uninterp` whose value is identical pre/post a concrete page-table write. There is **no spec link** from a concrete PTE write to global-view membership, so the `mapped.contains` postcondition is underivable.

#### 3. `identity_map_page` (identity_map.rs:718) — blocked by `accessible` postcondition

```
error: postcondition not satisfied        --> identity_map.rs:712:13   (x3 arms)
    |  identity_map_view().accessible(phys_addr@)   /   !accessible(...)
```
Same root cause: no spec relates the `KERNEL_PD_PADDR` atomic load + sub-call composition to the parameter-free `identity_map_view()`.

#### Additional limitation hit
`use_type_invariant(pde_idx)` on arch's `TableIndex` raises **`Verus Internal Error: missing type invariant function`** (cross-crate; documented at `phys.rs:137-156` and `verus-unsupported.md`). The supported workaround `TableIndex::into_raw()` is what I used above — it confirms the index bounds are *not* the wall; the ghost-token obligations are.

#### Why this is a guidance request, not a skill gap
Every analogous function in the completed sibling module `mm::phys` that references the parameter-free `phys_view()` (`frame::alloc/book/instance`, `kframe::new`) is `#[verus_verify(external_body)]` + listed in `tcb-allowed.md`, with the view-attachment lemmas (`lemma_manager_attached`) left as deferred axioms. Notably, `kframe::new` is TCB-listed **specifically because** it calls `identity_map_page` and needs the unrealized `mm::virt` token. The spec phase placed these three functions in-scope with `admit()` anticipating that same token would later be realized.

**Decision needed:** to reach `admit=0` for the exec trio, one of the following — both outside this in-body, fixed-signature module task — must happen:
- **(a)** Realize the `mm::virt` identity-map ghost token: a `bump_view(...).inv()` attachment lemma for `PAGE_TABLE_ALLOCATOR`, plus spec links from `Table::write`/atomic loads to `identity_map_view()`. This is cross-cutting infra spanning `bump_allocator`, `arch::paging`, and `mm::virt`.
- **(b)** Reclassify the three exec functions as TCB `external_body` (mirroring their `mm::phys` analogues) — which requires adding them to `tcb-allowed.md`, currently **forbidden** by the proving rules.

I have left the three `proof! { admit(); }` lines in place so the module stays green (9 verified, 0 errors) and the build passes, pending your direction on (a) vs (b).
