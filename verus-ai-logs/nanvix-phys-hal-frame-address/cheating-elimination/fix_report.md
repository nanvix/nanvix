# Cheating Elimination Report: hal-frame-address

Scope: `src/kernel/src/hal/mem/types/address/frame.rs` (+ `frame.spec.rs`,
`frame.proof.rs`). In-scope functions: `FrameAddress::into_raw_value`,
`FrameAddress::into_frame_number`, `FrameAddress::from_raw_value`,
`FrameAddress::from_frame_number`, and the `FrameAddress` struct/`View`/`inv`.
Out-of-scope (untouched): `new`, `into_physical_address`, `into_page_address`,
`fmt`, `eq`, and every other kernel module.

## Cheating Counts (before → after) — in-scope frame module only
| Item | Before | After | Eliminated |
|------|--------|-------|------------|
| admit() | 0 | 0 | 0 |
| assume() | 0 | 0 | 0 |
| external_body | 0 | 0 | 0 |
| assume_specification | 3 (all TCB-allowed) | 3 (all TCB-allowed) | 0 |
| cfg-gated exec | 0 | 0 | 0 |

Supporting governed declarations (not gate-flagged cheating, all registered in
`verus-ai-logs/tcb-allowed.md`):
- `axiom fn lemma_phys_view_is_spec_addr` (`frame.proof.rs:38`) — view/spec_addr
  bridge, external-bottom trust boundary.
- `uninterp spec fn spec_page_size()` (`frame.spec.rs:42`) — external-bottom
  canonical frame size; carries no body.

The automated cheating gate (`guardrails.detect_cheating`) scans for `assume(`,
`#[verifier::external_body]`, `admit(`, `#[verifier::trusted]`, and
`exec_allows_no_decreases_clause`. The module scan returns
**"✅ No cheating detected in module hal::mem::types::address::frame"**.
`assume_specification` and `axiom fn` are governed trust-boundary mechanisms (not
in `CHEATING_PATTERNS`); each frame-module instance is explicitly approved in
`tcb-allowed.md`.

Crate-global gate (`external_body=24 cfg_gate=6`, `assume=0 admit=0 trusted=0`)
is entirely out-of-scope: the FrameAddress module contributes **0** entries to
`cheating-detail.txt` (confirmed by `grep "address/frame"` → none). Those counts
belong to sibling/other modules owned by their own verification tasks
(`page.rs`, `phys.spec.rs`, `mm/phys/*`, …) and are not touched here.

## Items Eliminated
No in-scope item required elimination in this phase: the proving phase already
delivered a clean module. The one cheating item the base branch carried was
eliminated upstream and is verified absent here:

- `FrameAddress::into_raw_value` — **was** `#[verus_verify(external_body)]` on the
  base branch (`verus-ai/sys-virt-address`, line 95). It is now **body-verified**
  with the identical exec body (`self.0.into_raw_value()`) against the contract
  `result as int == self@`. The `external_body` is gone; the module verifies
  (exit 0). This is a genuine elimination, confirmed by the base-branch diff.

The remaining trust boundaries are irreducible within this module's scope and are
all governed in `tcb-allowed.md`:

- `assume_specification[ ::arch::mem::PAGE_SIZE ]` (`frame.spec.rs:45`) —
  foreign `arch` runtime constant; `arch` is not Verus-enabled. Established
  library-edge boundary (`tcb-allowed.md`, referenced repeatedly as precedent).
- `assume_specification[ <PhysicalAddress as Address>::from_raw_value ]`
  (`frame.spec.rs:110`) — TCB-allowed (`tcb-allowed.md:285`). The
  `impl Address for PhysicalAddress` cannot be body-verified in place: its sibling
  methods contain `usize as *const u8` / `usize as *mut u8` casts the Verus
  front-end rejects (see `verus-unsupported.md`), and per-method `external_body`
  would pull the whole `impl` into scope.
- `assume_specification<T: Address>[ <PageAligned<T> as Deref>::deref ]`
  (`frame.spec.rs:129`) — TCB-allowed (`tcb-allowed.md:301`). Auto-deref of the
  external `core::ops::Deref` trait; pure projection
  (`spec_addr(result) == addr@`).
- `axiom fn lemma_phys_view_is_spec_addr` (`frame.proof.rs:38`) — TCB-allowed
  (`tcb-allowed.md:333`). Relates a `PhysicalAddress`'s `@` to the universal
  `spec_addr` projection; not derivable because `spec_addr<T: Address>` is
  `uninterp` (a bare `T: Address` carries no `View<V = int>` bound). Discharged
  with the governed `axiom fn` mechanism (no `admit`, no `external_body`).

### Escalation-ladder evidence (verus-constraints)
The kept boundaries cannot be discharged inside this module — confirmed, not
assumed:
1. **vstd search**: there is no vstd spec for `::arch::mem::PAGE_SIZE`,
   `PhysicalAddress`, or `PageAligned<T>` — they are workspace-internal /
   foreign-crate items, not std/core types vstd models.
2. **Root cause is out of scope**: discharging
   `<PhysicalAddress as Address>::from_raw_value` / the `Deref` boundary / the
   view↔spec_addr bridge requires verifying `impl Address for PhysicalAddress`,
   which is blocked today by `usize as *const/*mut u8` casts Verus rejects
   (`verus-ai-logs/verus-unsupported.md`). That is the `hal::mem` verification
   task, not this module.
3. **Equivalent rewrite**: the `let x = complex_expr; f(x)` rewrites in the three
   constructors/projections are the pre-approved exec deviation that lets the
   bridge lemma relate `@` and `spec_addr`; no rewrite removes the boundary
   itself without verifying the foreign `Address` impl.

## Verification TODOs (verus-ai-logs/nanvix-phys-hal-frame-address/verification_todo.md)
None. There are zero proof gaps: no `admit()`, no `assume()`, no proof-gap
placeholders. Every in-scope function is body-verified against its
`#[verus_spec]` contract and the module verifies with exit code 0
(status CLEAN). No `verification_todo.md` entry is required.

## AST Consistency
- Tool: `scripts/ast_consistency.py --base-ref verus-ai/sys-virt-address`.
- Result: matched=6, mismatched=3, missing=0, extra=0.
- The 3 exec MISMATCHes (`from_frame_number`, `from_raw_value`,
  `into_frame_number`) are each the **pre-approved** deviation
  `f(complex_expr)` → `let x = complex_expr; f(x)` (ast-consistency skill table),
  documented inline with a `VERUS DEVIATION (pre-approved: ...)` comment. The
  bound local is an intermediate value for the ghost bridge lemma; the `proof!`
  blocks are ghost (stripped by the exec-only checker) and do not appear in the
  exec diff. Each rewrite preserves semantics (`PhysicalAddress` is `Copy`; same
  value, same evaluation order, same call target), time complexity, and space
  complexity.
- Zero unexplained mismatches confirmed: **YES**.

## Result: PASS
