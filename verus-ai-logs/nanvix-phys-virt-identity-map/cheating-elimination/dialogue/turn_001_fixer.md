## Response to Turn 1

### What I Changed

I attempted real proofs for all three functions (not justification). Concretely I
tried, in order:

1. **Discharge the `read` index preconditions** with
   `proof! { use_type_invariant(pde_idx); }` / `use_type_invariant(pte_idx);` at
   `identity_map.rs:534` / `:632`.
2. **Plain admit removal** (deleted all three `proof! { admit(); }`) and let Verus
   report the residual obligations with full error text.
3. **Make `identity_map_view()` interpretable** — changed
   `identity_map.spec.rs:36` from
   `pub uninterp spec fn identity_map_view() -> IdentityMapView;` to a concrete
   `pub open spec fn identity_map_view() -> IdentityMapView { IdentityMapView {
   initialized: false, mapped: Set::empty() } }` and re-ran (to try to discharge the
   membership/accessibility postconditions the reviewer asked for, via the proof.rs
   lemmas, without weakening a contract).

All three attempts failed at obligations that are **out of this module's scope to
discharge**. After capturing the exact errors I restored the file to the
byte-identical pre-task state (the 3 documented `proof! { admit(); }` at
`:534/:632/:719`; `identity_map.spec.rs` unchanged from pristine `6a31083cd`).

I did **not** weaken any contract, add `assume()`, add an unlisted `external_body`,
or change a function signature.

### Verification

Current restored state (what is committed):

```
$ make verify-kernel
=== Results ===
  76 verified
  0 errors
  Exit code : 0
  cheating: assume=0 external_body=12 admit=31 trusted=0 no_decreases=0 cfg_gate=15
  status: CHEATING_DETECTED   (3 in-scope admits at identity_map.rs:534/632/719)

$ make all        # erased-exec build (admits are cfg-gated ghost, erased)
Standalone images built successfully.   # exit 0
```

The reviewer's bar — `admit=0` **with** `Exit code : 0` — is **not simultaneously
achievable**: removing the admits makes `make verify-kernel` exit 101 (3 errors).

### Result: NEEDS_GUIDANCE

The blockers are not in the three target functions; they are three out-of-scope
dependency facts. Exact errors from each approach below.

---

#### Approach 1 — `use_type_invariant` to discharge the `read` index preconditions

`Table::read` requires `index@ < PAGE_TABLE_LENGTH`. `TableIndex` carries that as a
type invariant in the **arch** crate, but it is not usable cross-crate:

```
error: Verus Internal Error: missing type invariant function
   --> src/kernel/src/mm/virt/identity_map.rs:534:33
```

So the index bound cannot be recovered inside `ensure_pt`/`ensure_pte`. The only sound
way to get it is to **add `requires pde_idx@ < PAGE_TABLE_LENGTH`** to `ensure_pt`'s
contract (and the `pte_idx` analogue to `ensure_pte`) — a **contract/signature
change** the Turn-1 constraint explicitly forbids ("contracts must remain as strong
as they are now"). It also would not unblock the postconditions (Approach 2/3).

#### Approach 2 — plain admit removal (real obligations surfaced)

`make verify-kernel MODULE=mm::virt::identity_map` → **6 verified, 3 errors** (exit 101):

```
error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:534:44      (pd.read(pde_idx))
   ::: src/libs/arch/src/x86/mem/paging/table.rs:205:13
205 |             index@ < crate::mem::PAGE_TABLE_LENGTH,   <- failed precondition

error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:549:9       (PAGE_TABLE_ALLOCATOR.alloc_as::<…>())
   ::: src/libs/bump_allocator/src/lib.rs:350:18
350 |   requires bump_view(self).inv(),                    <- failed precondition

error: postcondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:618:13
618 |  Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr as int)),
        ... at this exit: 639 | return Ok(());            (idempotent present-PTE fast path)

error: precondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:631:40      (pt.read(pte_idx))
   ::: src/libs/arch/src/x86/mem/paging/table.rs:205:13
205 |             index@ < crate::mem::PAGE_TABLE_LENGTH,   <- failed precondition

error: postcondition not satisfied                        (×3 exits)
   --> src/kernel/src/mm/virt/identity_map.rs:706:13
706 |  Ok(_) => identity_map_view().accessible(phys_addr@),
```

- **`bump_view(self).inv()` (line 549).** `bump_view` is
  `pub uninterp spec fn bump_view(...) -> BumpView` over the module-level `static
  PAGE_TABLE_ALLOCATOR`. `inv()` is a non-trivial conjunction of facts about that
  opaque value; nothing in `bump_allocator` (no lemma/axiom — `new()` has an empty
  `ensures`) establishes `inv()` for the static. By design its realization is
  "deferred to the proving phase" (lib.spec.rs:166-169). I cannot establish `inv()`
  of an uninterpreted value, and I am forbidden from adding an axiom/`assume_spec`.
- **`mapped.contains(...)` (618) / `accessible(...)` (706).** See Approach 3 — these
  are logically unprovable while `identity_map_view()` is a parameterless constant.

#### Approach 3 — make `identity_map_view()` concrete (to try to prove the postconditions)

Set `identity_map_view() = { initialized: false, mapped: Set::empty() }`, removed
admits, re-ran. Still **6 verified, 3 errors**:

```
error: postcondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:618:13
618 | Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr as int)),
        ... at exit 639 | return Ok(());
        # Set::empty().contains(p) == false  →  Ok-branch is unprovable

error: postcondition not satisfied
   --> src/kernel/src/mm/virt/identity_map.rs:706:13
706 | Ok(_) => identity_map_view().accessible(phys_addr@),
        ... at the Err exits (730 `ensure_pt(...)?`, end of body)
        # accessible = (!initialized || …) = (!false) = true  →  Err => !accessible
        #   becomes Err => false, unprovable
```

This is the decisive demonstration the reviewer asked for: a **concrete** view cannot
satisfy the contract either. The two functions each assert a **result-dependent pair**
over the *same* fixed view —
`ensure_pte`: `Ok => mapped.contains(p)` **and** `Err => !mapped.contains(p)`;
`identity_map_page`: `Ok => accessible(p)` **and** `Err => !accessible(p)`.
For any fixed value of `identity_map_view()`, `contains(p)`/`accessible(p)` has one
fixed truth value, so exactly one of the two clauses becomes `=> false`. The exec
return decision (PTE read result, frame-number range, `pd_paddr == 0`) is not tied to
that fixed set. **No proof, lemma (`lemma_install_page_maps`, `lemma_map_page_*`), or
`vstd` search can discharge both clauses** — it is a logical impossibility, not a
proof-engineering gap. (The proof.rs lemmas prove facts about `spec_install_page` /
`spec_map_page` *transitions* `v -> v'`, but the contracts are written over a single
`v` with no `old`/`new` distinction, so the lemmas have nothing to connect to.)

#### Why a sound full fix is out of scope

To make the postconditions provable, `identity_map_view().mapped` must reflect the
real page-table memory, and `ensure_pte`'s `pt.write(pte_idx, present_pte)` must be
shown to make that PTE present. But `Table::write`
(`src/libs/arch/src/x86/mem/paging/table.rs:246`) has **no contents `ensures`** —
deliberately: `tcb-allowed.md` records that pinning the pure global ghost
`spec_table_word(addr,index)` to the written `entry` on an `external_body` (assumed)
write is **unsound** (two writes of distinct entries to one slot would assume
`e1 == e2`, i.e. `false`). The genuine `old@ -> @` slot transition is therefore
deferred to the proving-phase page-table permission token. `table.rs` is **not in
scope** for this task, and I am forbidden from (a) changing exec signatures of the
out-of-scope callers (`init`/`ensure_identity_mapped_range` at line 134, `memset`,
`memcpy`) to thread a `Tracked` token, (b) adding an `external_body` not in
`tcb-allowed.md`, or (c) adding an `assume_specification`/`axiom`.

#### Specific guidance requested

The three target functions cannot reach `admit=0` without one of the following,
each currently out of this module's scope:

1. **A spec-readable model of the singleton state** — replace the `uninterp`
   `identity_map_view()` / `bump_view()` with a ghost token (`PointsTo` /
   atomic-ghost / global invariant) over `KERNEL_PD_PADDR` and `PAGE_TABLE_ALLOCATOR`.
   Requires exec changes to module statics and out-of-scope callers.
2. **A sound `old@ -> @` contents postcondition on `arch::…::Table::write`** (and the
   matching permission token), which `tcb-allowed.md` documents as deliberately
   deferred-because-unsound-for-`external_body`. `table.rs` is out of scope.
3. **Approval to add a precondition** `requires pde_idx@ < PAGE_TABLE_LENGTH` /
   `pte_idx@ < PAGE_TABLE_LENGTH` to `ensure_pt`/`ensure_pte` (to fix the line-534/631
   read preconditions only — does not touch the 549/618/706 blockers).

Items (1) and (2) are the shared "proving-phase ghost-token" framework every sibling
`mm::phys::*` singleton free function defers; building it is a cross-module change
beyond `mm::virt::identity_map`. Detailed reproduction recorded in
`verus-ai-logs/nanvix-phys-virt-identity-map/verification_todo.md`.
