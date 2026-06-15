## Response to Turn 2

### What I Changed

Mirrored the `kframe::new` precedent exactly: converted the exec trio to `external_body` + TCB-list, removed all `admit()`, removed cfg-gated exec, and fixed the trigger note.

**1. `src/kernel/src/mm/virt/identity_map.rs` — exec trio → `external_body`, `admit()` removed, cfg gates removed**

| Function | Line | Change |
|---|---|---|
| `ensure_pt` | 534 | Deleted `proof! { admit(); }`. Added `#[verus_verify(external_body)]` directly above `#[verus_spec(...)]`. Contract unchanged. |
| `ensure_pte` | 626 | Same: deleted `admit()`, added `#[verus_verify(external_body)]`. Contract unchanged. |
| `identity_map_page` | 715 | Same: deleted `admit()`, added `#[verus_verify(external_body)]`. Contract unchanged. |

Removed **all 5** `#[cfg(not(verus_keep_ghost))]` exec gates in this file (2 in `ensure_pt` body, 1 in `ensure_pt` PDE-frame closure, 2 in `ensure_pte`) so the bodies are plain exec. The erased build compiles clean (the `error!` macro calls are now unconditional in the real body).

**2. `src/kernel/src/mm/virt/identity_map.proof.rs` — trigger note fixed (line 28)**
Annotated the explicit trigger: `assert forall|x: int| #[trigger] v.mapped.contains(x) implies …`. The "automatically chose triggers … low confidence" note is gone.

**3. `verus-ai-logs/tcb-allowed.md` — new `mm::virt::identity_map` subsection**
Added concrete, verified justifications for all four in-scope `external_*` entries:
- `ensure_pt` — calls `PAGE_TABLE_ALLOCATOR.alloc_as`, whose `requires bump_view(self).inv()` (lib.rs:350) has **no** establishing lemma/`type_invariant` in `src/libs/bump_allocator/` (grep-verified: none). The `inv()` is the bump allocator's own deferred ghost token, not derivable from `ensure_pt`'s only precondition `identity_map_view().inv()`. Contract preserved: `Ok ⇒ inv() ∧ page-aligned(pt_paddr)`, `Err ⇒ inv()`.
- `ensure_pte` — installs the leaf PTE via contents-free `Table::write` (documented-unsound to give a contents `ensures`); the `mapped.contains` ↔ write linkage is the deferred page-table permission token. Contract preserved verbatim.
- `identity_map_page` — composes `KERNEL_PD_PADDR` atomic load + `ensure_pt`/`ensure_pte`; no spec links the composition to the parameter-free `identity_map_view()`. Cross-referenced that `kframe::new` is already TCB-listed *because it calls this function*. Contract preserved verbatim.
- `ExPageTableBss` (spec.rs:143) external-type registration added for completeness.

No spec weakened: spec-drift `git-diff` against HEAD reports **Contract drift: 0** (ensures removed: 0, requires added: 0).

### Verification

**`make verify-kernel` (full) → exit 0, no errors.** Cheating detail (`cheating-detail.txt`):
- In-scope `admit` for `mm/virt/identity_map*` = **0** (grep returns nothing — admit→0 in scope).
- The trio now appears as `external_body`, each with a 1:1 entry in `tcb-allowed.md`:
  ```
  - mm/virt/identity_map.rs:534 ensure_pt: external_body
  - mm/virt/identity_map.rs:626 ensure_pte: external_body
  - mm/virt/identity_map.rs:715 identity_map_page: external_body
  - mm/virt/identity_map.spec.rs:143 ExPageTableBss (struct): external_type_spec
  ```
- The only 4 remaining global `admit`s are pre-existing and **out of scope**: all in `mm/phys/manager.proof.rs` (`lemma_manager_attached`, `lemma_kernel_alloc_one`, `lemma_kernel_alloc_contiguous`, `lemma_user_bulk_err_restored`).
- No `automatically chose triggers` note for `identity_map.proof.rs` in the verus log (grep returns nothing).

Module-scoped (`make verify-kernel MODULE=mm::virt::identity_map`): `6 verified, 0 errors (exit 0)`, in-scope admit=0.

**Erased build** — `cargo build --locked -p kernel --target build/targets/x86-kernel.json -Zbuild-std=core,alloc,compiler_builtins -Zbuild-std-features=compiler-builtins-mem -Zjson-target-spec`:
```
Compiling kernel v0.16.17 (/home/ruize/nanvix-phy-specs/src/kernel)
    Finished `dev` profile [optimized + debuginfo] target(s) in 9.23s
```
**0 errors, 0 warnings.**

Spec-drift (`spec_drift.py git-diff src/kernel/src/mm/virt/identity_map.rs --before HEAD`):
```
- Functions with changes: 0
- Contract drift (⚠ review required): 0
  - Ensures removed: 0
  - Requires added: 0
✅ No contract drift detected.
```

### Result: FIXED

- **Zero remaining `admit()` in scope** — the exec trio is now `external_body` (the codebase-sanctioned ghost-token resolution), each TCB-listed 1:1 with concrete justification.
- **Zero `external_body` unlisted** — all 3 functions + `ExPageTableBss` are in `tcb-allowed.md`.
- **No cfg-gated exec code** — all 5 `#[cfg(not(verus_keep_ghost))]` gates removed from this module; erased build still clean.
- **0 errors / 0 warnings** — the `proof.rs:28` trigger note is gone.
- **No spec weakening** — every `requires`/`ensures` byte-identical; spec-drift exit 0.
- **No cross-module regression** — full `make verify-kernel` exit 0.
