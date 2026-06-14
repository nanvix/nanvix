## Turn 2: Verifying fixer turn 1 — admits (item 2) + remaining checklist

### Progress
- Done (verified PASS this turn): items 1, 3, 4, 5, 6, 7, 8, 9 (evidence below).
- Current: item 2 — Zero remaining admit() → **BLOCKED** (verified, not a justification dodge).
- Remaining/blocked: item 2 and item 10 (10 is gated by 2). Both rooted in the same 3 admits.

I did NOT accept the fixer's report at face value. The fixer made **no code change**
(NEEDS_GUIDANCE) and returned justifications. Per my rules, justification ≠ fix — so I
**independently reproduced** the two pivotal claims with my own edits + `make verify-kernel`,
then reverted. Results below.

### Verification

Baseline (clean working tree, restored): `make verify-kernel` →
`76 verified, 0 errors (exit 0)`, module `mm::virt::identity_map` = 9 verified, 0 errors,
0 warnings. Cheating tool: `assume=0 external_body=12 admit=31 cfg_gate=15` (crate-wide;
per-module rows below).

**Independent reproduction #1 — item 5 (cfg-gated `error!`).** I removed
`#[cfg(not(verus_keep_ghost))]` from the `error!("ensure_pt(): {reason}")` at line 537 and
ran `make verify-kernel`:
```
error: Unsupported constant type
  --> src/kernel/src/macros.rs:188 (crate::klog::KlogLevel::Error)
537 | error!("ensure_pt(): {reason}");
    = note: originates in macro `write` / expansion of macro `error`
error: could not compile `kernel` ... (verus did not run)  Exit code: 101
```
→ Verus genuinely **cannot compile** the `error!` macro. This is an isolated reproducer for
the specific construct (single ungated `error!`), satisfying item 7. The cfg-gate is therefore
**mandatory**, and the pattern is pervasive across every sibling verified module
(`frame.rs`: 22 gates, `manager.rs`: 9, `kframe.rs`/`upool.rs`: 1 each). The gated code is
pure logging with no effect on return value or abstract state → semantically equivalent.
**Item 5 = PASS (justified Verus limitation, reproduced).** Reverted.

**Independent reproduction #2 — item 2 (admit in `ensure_pte`).** I replaced
`proof! { admit(); }` at line 632 with a no-op and ran `make verify-kernel`:
```
error: postcondition not satisfied
 --> identity_map.rs:619  Ok(_) => identity_map_view().mapped.contains(spec_page_base(phys_addr as int))
error: precondition not satisfied
 --> identity_map.rs:633  pt.read(pte_idx)   (arch table.rs:205: requires index@ < PAGE_TABLE_LENGTH)
verification results:: 75 verified, 1 errors   Exit code: 101
```
→ Confirms the fixer's core claim. `identity_map_view()` is `uninterp spec fn ... ->
IdentityMapView` (spec.rs:36) — a **parameter-free opaque constant**. No exec input threads
into it, and `arch::…::Table::write` (per `verus-ai-logs/tcb-allowed.md`) **deliberately
exposes no `old@ -> @` contents postcondition** (a contents post on that `external_body` was
shown unsound). So there is literally no spec fact flowing from `pt.write(pte_idx, new_pte)`
to `identity_map_view().mapped`, and the `Ok`/`Err` membership obligations are
undischargeable from any exec body short of `admit`/`assume`. Reverted.

**Architectural blocker (verified real, not accepted on the fixer's word):** unblocking the
3 admits requires a proving-phase **ghost-token / page-table-permission** framework that:
1. attaches spec-readable ghost state to the `KERNEL_PD_PADDR`/`KERNEL_CR3` atomics —
   changing the `load`/`store` sites in **`init`** and **`sync_kernel_pdes`** (out of scope);
2. depends on `arch::…::paging::table::Table::write` exposing the slot-update postcondition
   that `tcb-allowed.md` records as **deliberately deferred** (`table.rs` out of scope);
3. would thread a `tracked` token through `ensure_pt`/`ensure_pte`/`identity_map_page`,
   changing their signatures and breaking out-of-scope callers (`init`, `memcpy`, `memset`,
   `ensure_identity_mapped_range`).

This is corroborated by: (a) spec.rs:32-35 and 56-62 explicitly state the `v -> v'`
transition "is realized in the proving phase by a ghost token over those singletons";
(b) `tcb-allowed.md` names this the "`identity_map_view()` `v -> v'` deferral convention";
(c) **every sibling `mm::phys::*` module carries the identical exec `admit()`s at the same
singleton/global-view boundary** (`frame.rs` alloc/free/etc., `manager.proof.rs` lemmas) and
was accepted as committed "verify PASS". The blocker is a **cross-module framework decision**,
not a local proof gap, and cannot be closed without editing out-of-scope modules/functions.

**Per-item determinations (module-scoped):**
| # | Item | Verdict | Evidence |
|---|---|---|---|
| 1 | No specs weakened | PASS | Contracts in identity_map.rs unchanged & full-strength; `admit` leaves them *unproven*, not *weakened*. Fixer touched no spec. |
| 2 | Zero admit() | **BLOCKED** | 3 admits (`ensure_pt:534`, `ensure_pte:632`, `identity_map_page:719`); removal reproduced as undischargeable in-scope. |
| 3 | external_body ⊆ tcb-allowed | PASS | Module's only occurrence = `ExPageTableBss` (spec.rs:141-143), an `external_type_specification` opaque-type marker for the BSS storage type — **no trusted exec body**. The 12 crate-wide `external_body` *functions* are all in siblings/arch and listed in tcb-allowed.md. |
| 4 | No assume/assume_specification (std/external only) | PASS | `assume=0`. 2 `assume_specification`s: `<[T]>::as_ptr` (std external-bottom) and `FixedSizeBumpAllocator::<N,A,S>::new` (lib.rs:243 has **no `#[verus_spec]`** — confirmed; external crate not yet verified). Both are external-bottom boundaries the item permits. |
| 5 | No cfg-gated exec | PASS | 5 × `#[cfg(not(verus_keep_ghost))]` over `error!`; ungating **reproduced** as Verus compile failure. Logging-only, semantically equivalent, sanctioned pattern. |
| 6 | Cheating audit counts | PASS (reported) | Module: `admit=3`, `external_body(type-spec)=1`, `assume_specification=2`, `cfg_gate=5`. Locations enumerated above. |
| 7 | Isolated reproducer for claimed limitation | PASS | Single ungated `error!` → "Unsupported constant type" (isolated). |
| 8 | Exec rewrites minimal | PASS | No `// VERUS REWRITE`; no exec rewrites present. |
| 9 | Cross-module regression | PASS | `make verify-kernel` (all) → 76 verified, 0 errors, exit 0; no sibling regressed. |
| 10 | 0 errors, 0 warnings | **BLOCKED** | 0 Verus errors/warnings, but `status: CHEATING_DETECTED` (admit=3). Gated entirely by item 2. |

### Fix Request (deferred — requires cross-module authorization)

Items 2 and 10 cannot be closed inside this isolated module/task. They are **not** fixable by
any in-scope edit, and demanding an in-scope "fix" would force a forbidden spec-weakening or an
out-of-scope edit. To unblock in a future cross-module effort, the following infrastructure
must land (NONE of which is in this task's scope):

1. A verified/ghost-tracked wrapper over `KERNEL_PD_PADDR`/`KERNEL_CR3` (mirroring
   `mm/phys/frame.rs::instance`'s singleton-to-`phys_view()` attachment) giving
   `identity_map_view()` a concrete, spec-readable body.
2. An `old@ -> @` slot-update postcondition on `arch::…::paging::table::Table::write`
   (currently deferred in `tcb-allowed.md`), so `pt.write`/`pd.write` move the view's `mapped`
   set — enabling the already-proven `proof.rs` lemmas (`lemma_install_page_maps`,
   `lemma_map_page_accessible`, `lemma_*_preserves_inv`) to fire from the exec bodies.
3. A `tracked` page-table permission token threaded through `init` → `ensure_pt` →
   `ensure_pte` → `identity_map_page`, updating callers `init`/`memcpy`/`memset`/
   `ensure_identity_mapped_range` in lockstep.

Until that framework exists, the 3 exec `admit()`s are the documented proving-phase deferral,
identical to every sibling `mm::phys::*` module. No spec was weakened; no
`assume`/`external_body`/`admit`-disguise was added to paper over the gap; baseline remains
`76 verified, 0 errors, exit 0`.

**STOP = BLOCKED** — item 2 (and consequently item 10) is genuinely stuck after multiple
attempts (fixer attempts A–E + my two independent reproductions), with the blocker verified to
require out-of-scope cross-module infrastructure. All other checklist items (1, 3, 4, 5, 6, 7,
8, 9) are PASS with concrete evidence.
