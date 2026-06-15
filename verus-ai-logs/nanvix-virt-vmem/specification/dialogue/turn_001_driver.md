## Turn 1: Full checklist walk-through (specification phase, `vmem`)

### Progress
- Done (PASS): View-consistency (item 3, partial), No-tautological-ensures (item 4, but see caveat), No-assume_specification-internal (item 7), vstd-searched (item 8)
- FAIL (blocking): item 1 (fn coverage), item 2 (caller coverage), item 6 (error-path ensures), item 12 (loop invariants), item 13 (no cheating: admit/external_body), item 15 (build), item 16 (verify-all cross-module), item 17 (verification)
- Not fully assessable until code compiles: items 5, 10, 11, 14
- Current: the phase FAILS at the gate. Highest-priority fix is the compile error, then spec coverage + removal of `admit()`.

---

### Verification performed

Commands run:
- `make verify-kernel` → **VERIFY_FAILED**, exit 101. Verus did **not** run — two `rustc` type errors:
  ```
  error[E0308]: mismatched types
    --> src/kernel/src/mm/virt/vmem.rs:196:95   VmemView { ... pgdir: 0 }  expected `nat`, found integer
    --> src/kernel/src/mm/virt/vmem.rs:260:95   VmemView { ... pgdir: 0 }  expected `nat`, found integer
  Results: 0 verified, compilation/setup error (verus did not run)
  ```
- Guardrail summary from the same run:
  - `coverage: 2/1069 exec functions have contracts`
  - `cheating: assume=0 external_body=29 admit=2 trusted=0`
  - In-scope cheating (`verus-logs/cheating-detail.txt`):
    ```
    mm/virt/vmem.rs:124 new: admit
    mm/virt/vmem.rs:212 clone: admit
    mm/virt/vmem.rs:372 allocate_kernel_page_table: external_body
    mm/virt/vmem.rs:393 allocate_user_page_table: external_body
    mm/virt/vmem.rs:526 is_kernel_addr: external_body
    mm/virt/vmem.rs:545 is_kernel_region: external_body
    mm/virt/vmem.rs:599 lookup_user_page_table: external_body
    mm/virt/vmem.rs:644 lookup_kernel_page_table: external_body
    mm/virt/vmem.rs:698 find_user_frame: external_body
    mm/virt/vmem.rs:940 replace_user_page_cow_frame: external_body
    ```
- Read `vmem.rs`, `vmem.spec.rs`, `vmem.proof.rs`, `caller_analysis.md`, `view_design.md`.

Only **two** in-scope exec functions carry `#[verus_spec(... ensures ...)]`: `new` (L114) and `clone` (L201). Both end their body with `proof! { admit(); }` (L189, L253), discharging their own postconditions by cheating. `vmem.proof.rs` is empty (`verus! { }`).

---

### Per-item findings

**Item 1 — Every in-scope exec function has requires/ensures: FAIL.**
`view_design.md` enumerates 25 top-level entry points (`new`, `clone`, `load`, `pgdir`, `map_kpage`, `map`, `is_user_page_mapped`, `is_user_addr`, `is_user_region`, `is_physical_region`, `try_find_user_pte`, `for_each_user_mapping`, `mark_user_page_cow`, `unmark_user_page_cow`, `resolve_cow_at`, `resolve_cow_for_region`, `user_vaddr_to_paddr`, `copy_from_user_unaligned`, `copy_to_user_unaligned_unchecked`, `copy_to_user_unaligned`, `copy_user_to_user`, `memset`, `unmap`, `uctrl`, `kctrl`) with concrete spec sketches. Only `new` and `clone` are annotated. **23 of 25 entry points have no `requires`/`ensures`.** Guardrail confirms `2/1069`.

**Item 2 — Caller coverage: FAIL.**
`caller_analysis.md` records concrete caller expectations for every entry point (e.g. `map` → `Err` ⟹ `self@ == old@` and frame dropped, so fork rollback stays refcount-balanced; `unmap` → `Ok(Some(f))` ⟹ returned frame `== old@.user[v].frame` and `self@ == old@.spec_unmap(v)`; `resolve_cow_at` idempotency; `kctrl`/`copy_to_user_unaligned_unchecked` dry-run⇒commit). None of these are expressed as contracts (the functions are unspecified). The two specs that exist (`new`, `clone`) match their caller rows, but 23 caller expectations are unmet.

**Item 3 — View consistency: PASS (for what exists).**
`new`/`clone` ensures reference `VmemView` fields (`v@.user`, `v@.kernel`, `from@.kernel`) and assert `v.inv()`. `vmem.spec.rs` defines `VmemView`, `inv()`, observers, and all `spec_*` transitions, consistent with `view_design.md`. (Cannot be fully validated by Verus until item 15 is fixed.)

**Item 4 — No tautological ensures: FAIL (with the compile fix this becomes the dominant smell).**
Both `new` and `clone` use `Err(_) => true`. Item 6 below classifies this; per `caller_analysis.md` `new`/`clone` callers do allocate and need at least the standard frame fact, but a bare `true` is the textbook tautology the checklist forbids. Treat as FAIL pending item 6.

**Item 5 — No subsumed ensures: NOT ASSESSABLE.** Too few specs to evaluate redundancy; revisit after coverage is added.

**Item 6 — Error paths have meaningful ensures: FAIL.**
Every annotated function uses `Err(_) => true`. `caller_analysis.md` requires real error-path facts, e.g. `map`/`map_kpage`/`unmap`/`uctrl`/`kctrl` `Err` ⟹ `self@ == old@` (no partial state); `copy_to_user_unaligned` `Err` ⟹ nothing observable changed. `Err(_) => true` discards these.

**Item 7 — No assume_specification for workspace-internal code: PASS.** `grep` finds no `assume_specification` anywhere in the module. (External dep *types* use `external_type_specification`, which is the sanctioned mechanism for opaque lower-crate types — acceptable.)

**Item 8 — vstd searched before assume_specification: PASS (vacuous).** No `assume_specification` present.

**Item 9 — Specs written for the caller: FAIL.** Only because 23/25 specs are absent; the two present are caller-usable. Resolved when item 1 is.

**Item 10 — Trait obligations satisfied: FAIL.** `caller_analysis.md` flags `impl Drop for Vmem` (L1815) as the key trait obligation (release every owned frame/page-table/pgdir, no leak/double-free). `drop` has no spec and the View carries no ownership token that `Drop` could be specified against. At minimum the spec must state what `Drop` is trusted to do; currently nothing.

**Item 11 — Spec completeness (advisory): DEFERRED.** Re-run after item 1; intentional nondeterminism (e.g. `resolve_cow_at` existential frame) is acceptable per the sketches, but cannot be judged with 2 specs.

**Item 12 — Loop invariants: FAIL.**
Every exec loop must carry an `invariant` clause. `new` (L148, L185) and `clone` (L222, L236) contain `while let`/`loop` with **no** `invariant`. The bodies currently "pass" only because `proof! { admit(); }` poisons the path — that is cheating, not an invariant. Many other in-scope functions also loop (L330, L614, L662, L706, L748, L791, L827, L834, L1073, L1186, L1305, L1488) and will each need invariants once specced.

**Item 13 — No cheating on module's own functions: FAIL (10 instances; each must be addressed individually).**
The following are the module's **own** functions (not external deps) and are illegitimately trusted:
1. `new` (L124) — `admit`. Remove; prove the `Ok` arm against `inv()`.
2. `clone` (L212) — `admit`. Remove; prove the `Ok` arm against `inv()` and `v@.kernel == from@.kernel`.
3. `allocate_kernel_page_table` (L372) — `external_body`, no contract.
4. `allocate_user_page_table` (L393) — `external_body`, no contract.
5. `is_kernel_addr` (L526) — `external_body`, no contract. This is pure address arithmetic; it should be verified (or at least given `ensures result == spec_is_kernel_addr(virt_addr.addr_nat())`).
6. `is_kernel_region` (L545) — `external_body`, no contract. Should be `ensures result == spec_is_kernel_region(...)`.
7. `lookup_user_page_table` (L599) — `external_body`, no contract.
8. `lookup_kernel_page_table` (L644) — `external_body`, no contract.
9. `find_user_frame` (L698) — `external_body`, no contract.
10. `replace_user_page_cow_frame` (L940) — `external_body`, no contract.

`assume_specification` on genuinely-external lower-crate items is temporarily allowed, but **none of the above are external** — they are this module's private helpers. Each must either be verified, or (if a justified trust boundary) be given a real `ensures` contract rather than a bare `external_body`. `external_type_specification` on `Ex*` dep types (spec.rs) and `kpanic_unreachable` (macros.rs, out of scope) are not counted here.

**Item 14 — No specs weakened (spec-drift): PASS (vacuous, by `caller_analysis.md`).** Pre-existing `vmem.spec.rs` was empty and no prior `#[verus_spec]` existed (clean slate). Nothing upstream to weaken. Re-confirm after coverage lands.

**Item 15 — Build: FAIL.** `make verify-kernel` aborts at `cargo check` with the two `E0308` errors above; a plain build hits the same code. Nothing else can be verified until this compiles.

**Item 16 — Cross-module regression (verify-all): FAIL.** `make verify-kernel` verifies the whole `kernel` crate and it reports `0 verified, compilation/setup error`. Because the crate does not compile, **all** previously-verified modules are also blocked, not just `vmem`.

**Item 17 — Verification: FAIL.** `0 verified`, exit 101. See item 15.

---

### Fix Request (do these in order; re-run `make verify-kernel` after each milestone)

**FR-1 (blocking compile). Fix the `nat` literal type error.**
In `src/kernel/src/mm/virt/vmem.rs` at **L196** and **L260**, change `pgdir: 0` to a `nat` literal:
```rust
vmem_view: Ghost::new(VmemView { user: Map::empty(), kernel: Map::empty(), pgdir: 0nat }),
```
(or `pgdir: 0 as nat`). Verify: `make verify-kernel` must get past `cargo check` (Verus actually runs). This is also a *semantic* bug to flag separately (FR-5): a placeholder `pgdir: 0` is not the real page-directory base and cannot satisfy `inv()`'s `spec_is_physical_region(self.pgdir, page_size())` / `is_page_aligned` without `admit()`.

**FR-2 (remove cheating). Delete the two `admit()`s and contract the 8 `external_body` helpers (item 13).**
- Remove `proof! { admit(); }` at L189 (`new`) and L253 (`clone`). Prove the `Ok` arms against `inv()`; this requires the ghost `vmem_view` to actually reflect the constructed kernel map and real `pgdir` base (couples with FR-1/FR-5), and loop invariants (FR-4).
- For each of `allocate_kernel_page_table`, `allocate_user_page_table`, `is_kernel_addr`, `is_kernel_region`, `lookup_user_page_table`, `lookup_kernel_page_table`, `find_user_frame`, `replace_user_page_cow_frame`: either remove `external_body` and verify the body, or, if a justified trust boundary, replace the bare `external_body` with an `external_body` **plus a real `ensures`** that captures the helper's contract (e.g. `is_kernel_addr` → `ensures result == spec_is_kernel_addr(virt_addr.addr_nat())`; `find_user_frame` → relate `Ok` to `self@.user[page_base(v)].frame`). A bare `external_body` with no `ensures` is unconstrained and unacceptable.
Verify: `cheating-detail.txt` must show **no** `admit` and no contract-less `external_body` for module-owned functions.

**FR-3 (coverage, items 1/2/6/9/10). Add `#[verus_spec]` requires/ensures to the 23 missing entry points**, transcribing the sketches in `view_design.md` (§"Spec sketches per top-level entry point") and the caller rows in `caller_analysis.md`. Each must:
- match-style ensures `Ok => ... , Err => ...` with **meaningful** error facts (item 6) — e.g. mutators get `Err` ⟹ `self@ == old@`; never `Err(_) => true` where the caller relies on no-partial-state.
- reference `VmemView` fields / `spec_*` transitions (e.g. `map` ⟹ `self@ == old@.spec_map(v, frame, perms)`; `unmap` `Ok(Some)` ⟹ `self@ == old@.spec_unmap(v)` and returned frame `== old@.user[v].frame`; `resolve_cow_at` idempotency; `kctrl`/`copy_to_user_unaligned_unchecked` dry-run⇒commit).
- give `Drop` a trusted spec or document the ownership obligation (item 10).
Verify: guardrail `coverage` must rise from `2/...` to cover all 25 in-scope entry points; spot-check each ensures against its `caller_analysis.md` row.

**FR-4 (item 12). Add an `invariant` clause to every exec loop** that is in a specced/verified function — start with the four loops in `new` (L148, L185) and `clone` (L222, L236), then the loops in each newly-specced function. Define non-trivial invariants as `spec fn` in `vmem.spec.rs`/`vmem.proof.rs`. Verify: Verus must not emit "loop without invariant" and the functions must verify without `admit()`.

**FR-5 (bug awareness, item 14-adjacent). Record the `pgdir: 0` placeholder as a bug.**
The ghost `vmem_view` in both `new` and `clone` is constructed as `{ user: empty, kernel: empty, pgdir: 0 }`, which (a) does not reflect the kernel mappings `clone` claims (`v@.kernel == from@.kernel` is currently only "true" via `admit`), and (b) sets `pgdir` to `0`, violating `inv()`. Write this to the bugs file (`verus-ai-logs/nanvix-virt-vmem/bugs.md`) and fix the ghost construction to mirror the real built state.

### STOP / ROLLBACK
Not creating `STOP` — items 1, 2, 5–6, 9–17 are unresolved. Not requesting `ROLLBACK`: the View design (`view_design.md`) is sound and sufficient; every failure here is fixable in the specification phase (compile fix, add contracts, remove `admit`/`external_body`, add loop invariants). The fix belongs in this phase, not view-design.
