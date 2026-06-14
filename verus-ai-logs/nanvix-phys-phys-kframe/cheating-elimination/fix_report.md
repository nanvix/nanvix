# Cheating Elimination Report: phys-kframe

Scope (verification-order target functions): `KernelFrame::new`, `KernelFrame::drop`,
`KernelFrame::base`. Files I may modify: `kframe.rs`, `kframe.spec.rs`, `kframe.proof.rs`.
Hard rule in effect: **do not touch unlisted functions** (frame.rs, manager.*, mod.rs,
upool.rs are owned by other phases).

## Response to the round-2 grader demands

The grader's `cheating_report_1.md` is a **directory-wide** scan of `src/kernel/src/mm/phys/`.
Its counts (`external_body: 16 (15 user fns), admit: 12`) are dominated by functions in
OTHER modules. Mapped to owning phases (verified with `grep -nE "admit|external_body"`):

| Source line | Type | Owning phase | In phys-kframe scope? |
|-------------|------|--------------|------------------------|
| frame.rs:137,214,299,380,443,498,536,587 (8×) | admit | `phys-frame` | NO |
| manager.proof.rs:16,35,55,216 (4×) | admit | `phys-manager` | NO |
| frame.rs:668,702,758,786,815,831,870,889 (8×) | external_body | `phys-frame` | NO |
| manager.rs:96,524 (2×) | external_body | `phys-manager` | NO |
| mod.rs:59,87 (2×) | external_body | `phys-mod` | NO |
| mod.spec.rs:66 (ExLinkedList) | external_type_spec | `phys-mod` | NO |
| upool.rs:246,272 (2×) | external_body | `phys-upool` | NO |
| **kframe.rs:81 `KernelFrame::new`** | **external_body** | **phys-kframe** | **YES (TCB-allowed)** |

The **only** phys-kframe entry is `kframe.rs:81`. The hard rule forbids modifying any of the
other 27 entries; they belong to the `phys-frame`/`phys-manager`/`phys-mod`/`phys-upool`
phases. (Identical situation to `phys-upool`'s round-3 report, which carried the same list.)

### Demand 1 — "`admit()` and `assume()` must be replaced with real proofs"
**Not applicable to kframe scope.** `grep -nE 'admit\(|assume\('` over `kframe.rs`,
`kframe.spec.rs`, `kframe.proof.rs` returns **0 statements**. (The token "assume" appears once
as English prose in a doc comment at `kframe.spec.rs:8` — "…arithmetic behind it **assume** the
returned…" — not an `assume()` call.) `base` and `drop` are proven **in-body**; the module
verifies at **42 verified, 0 errors**.

### Demand 2 — "`trusted` and `external_body` on proof fns must be removed and the proof completed"
**Not applicable to kframe scope.** `kframe.proof.rs` is **empty** (`verus! { }`): zero proof
functions, hence zero `trusted`/`external_body` on proof fns. (The word "trusted" appears once
as prose in a code comment at `kframe.rs:80`, not as a `#[trusted]` annotation.) The single
`external_body` in scope is on the **exec** fn `KernelFrame::new`, which is TCB-allowed (see
below), not a proof fn.

### Demand 3 — "Multi-line `limitation_assume` bodies → single-line proposition (R20c)"
**Not applicable to kframe scope.** `grep -nE 'limitation_assume'` over the three kframe files
returns **0**.

### Demand 4 — "`#[verifier::exec_allows_no_decreases_clause]` (R20p) removed + real `decreases`"
**Not applicable to kframe scope.** `grep -nE 'exec_allows_no_decreases|no_decreases'` returns
**0**. `new`, `base`, `drop` contain no loops and no recursion (confirmed by inspection), so no
`decreases` clause is required or applicable.

## The one in-scope item: `KernelFrame::new` `external_body` (kframe.rs:81) — irreducible

This `external_body` is explicitly authorized in `verus-ai-logs/tcb-allowed.md` (both the
"Allowed `external_body`" section and the cross-module-dependency section). It is **provably
irreducible within `mm::phys`**, with evidence:

1. The body calls `crate::mm::virt::identity_map_page(phys_addr)`, whose precondition is the
   **global** `identity_map_view().inv()` (`src/kernel/src/mm/virt/identity_map.rs:511`).
2. `identity_map_view()` is declared `pub uninterp spec fn identity_map_view() -> IdentityMapView`
   (`identity_map.spec.rs:36`). Being **uninterp**, its value — and therefore `.inv()` — is
   completely opaque; it cannot be derived from `KernelFrame::new`'s only precondition
   (`base.inv()`) or from anything available in `mm::phys`.
3. `mm::virt` exports no lemma that *establishes* `identity_map_view().inv()` unconditionally.
   The only relevant lemmas — `lemma_install_page_preserves_inv`,
   `lemma_map_page_preserves_inv` (`identity_map.proof.rs:32,53`) — merely **preserve** `inv()`
   given a prior `inv()`; they cannot bootstrap it. The establishing fact is owned by the
   `mm::virt` identity-map ghost token, not realized in `mm::phys`.
4. `identity_map_page` itself is not yet verified (`identity_map.rs:718 identity_map_page: admit`,
   owned by the `virt-identity-map` phase), reinforcing that the dependency is still open.

Removing the `external_body` would force an in-body `assume(identity_map_view().inv())` — a
**strictly worse** cheat — so it is sound to keep it as the documented trust boundary. The
contract is non-trivial: `requires base.inv()`, `ensures Ok(kf) => kf@ == base@ && kf.inv()`.
It is eliminated (verified in-body) only when `mm::virt`'s identity-map token is realized —
the `virt-identity-map` phase's responsibility.

There is **no unnecessary/removable** `external_body` in kframe (unlike `phys-upool`, which had a
removable struct attribute): the `KernelFrame` struct uses `external_derive` for `#[derive(Debug)]`
(not counted by the gate, required for the derive), and `new`'s is design-mandated.

## Cheating Counts (phys-kframe scope, before → after)

| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 1 (TCB-allowed) | 1 (TCB-allowed) | 0 (irreducible) |
| assume_specification | 0 | 0 | 0 |
| trusted (annotation) | 0 | 0 | 0 |
| limitation_assume | 0 | 0 | 0 |
| exec_allows_no_decreases | 0 | 0 | 0 |
| cfg-gated exec | 1 (logging) | 1 (logging) | 0 (pre-approved) |

The one cfg-gate is `#[cfg(not(verus_keep_ghost))]` on the `error!("failed to free kernel
frame…")` log line in `KernelFrame::drop` — the identical pre-approved logging convention used by
the verified sibling `UserFrame::drop` (`upool.rs:205`). Not exec logic; the deallocation and all
Rust-visible behavior are identical across builds.

## AST Consistency
- `git diff verus-ai-prove -- kframe.rs kframe.spec.rs kframe.proof.rs` → **empty** (byte-identical
  to base). No exec code changed, no cfg gate added, no `external_body` introduced.
- Zero mismatches confirmed: **YES** (semantics/time/space trivially preserved).

## Verification
- `make verify-kernel MODULE=mm::phys` → **42 verified, 0 errors** (exit 0). All proof
  obligations for `KernelFrame::base`, `KernelFrame::drop`, and every other obligation discharge.
- The directory-scoped gate reports `CHEATING_DETECTED` solely from the 27 out-of-scope entries
  in `frame.rs`/`manager.*`/`mod.rs`/`upool.rs`, owned by other phases and protected by the hard
  rule. No phys-kframe change can lower that directory count.

## Verification TODOs (verus-ai-logs/nanvix-phys-phys-kframe/verification_todo.md)
- No genuine in-scope proof gaps. The single recorded item is the TCB-allowed cross-module trust
  boundary `KernelFrame::new` (`external_body`), eliminated when `virt-identity-map` realizes the
  `identity_map_view()` token. No `admit()`/`assume()` exist in any kframe file.

## Result: PASS (within phys-kframe scope)
Within the permitted scope, all cheating is eliminated or authorized: kframe has zero
`admit`/`assume`/`trusted`/`limitation_assume`/`exec_no_decreases`, and its single `external_body`
(`KernelFrame::new`) is TCB-allowed and provably irreducible in `mm::phys`. The residual
directory-gate count is entirely out-of-scope (other phases), which the hard rule forbids this
phase from touching.
