## Turn 1: cfg-gated exec code (#5) and external_body governance (#6/#9)

### Progress
- Done (PASS):
  - [x] Zero `admit()` — `assume=0 ... admit=0` in `make verify-kernel`; `detect_cheating` on all three phys files reports `admit=0`.
  - [x] Zero `assume()` — `assume=0` globally and per-file.
  - [x] Zero trusted functions — `trusted=0`.
  - [x] Zero `exec_allows_no_decreases_clause` — `no_decreases=0`.
  - [x] AST consistency — only 2 MISMATCH, both the **pre-approved** `f(complex_expr)` → `let x = complex_expr; f(x)` deviation (`from_number`, `into_frame_number`), each carrying a `VERUS DEVIATION (pre-approved: ...)` comment. Semantically equivalent.
  - [x] All exec rewrites commented — both rewrites carry the deviation comment; pre-approved deviations need no minimal reproducer per the ast-consistency skill.
  - [x] No specs weakened — `spec_drift.py` shows **0** contract drift vs cheating-elimination start (`27abdb10`), specification boundary (`cdbf5ab8`), and proving boundary (`f0e04979`).
  - [x] Cross-module regression — `make verify-kernel` exit 0 (4 verified, 0 errors).
  - [x] Verification + build — `make verify-kernel` exit 0; `./z build -- all` finished clean (kernel compiled, 0 errors/0 code warnings).
- Current: **#5 cfg-gated exec** and **#6/#9 external_body governance** (single shared root cause).
- Remaining: none unverified — only the two FAILs above need a fix.

### Verification

**#5 — Zero cfg-gated exec code (only imports/derives/debug_assert/logging allowed): FAIL**

`make verify-kernel` reports `cfg_gate=9`. Classifying every `#[cfg(verus_keep_ghost)]` in the phys module against `count_cfg_gates()` in `scripts/verify.sh`:

```
phys.rs:9   -> include!("phys.spec.rs");   ALLOWED (include!)
phys.rs:11  -> include!("phys.proof.rs");  ALLOWED (include!)
phys.rs:30  -> use ::vstd::prelude::*;     ALLOWED (use)
phys.spec.rs:21 -> verus! {                COUNTED  <-- violation
```

`phys.spec.rs:21` `#[cfg(verus_keep_ghost)]` immediately precedes `verus! {`. The
target is `verus! {`, which is **not** in the allowed set (`use`/`include!`/`extern`/`mod`/logging/`debug_assert`/`derive`/`feature`), so it is counted as cfg-gated exec. This gate is **redundant**: `phys.spec.rs` is already textually pulled in only via `#[cfg(verus_keep_ghost)] include!("phys.spec.rs")` at `phys.rs:9`, so its entire body — including the `verus!` block — is already compiled only under `verus_keep_ghost`. Note `phys.proof.rs` correctly uses a bare `verus! { }` with **no** inner cfg, and `mm/phys/mod.spec.rs` uses a bare `verus! {` as well. `phys.spec.rs` is the outlier.

**#6 / #9 — external_body governance: FAIL (two distinct defects)**

(a) **Misclassification caused by the same cfg gate.** Per-file `detect_cheating` (the function that produces the official `external_body=25` count):

```
phys.spec.rs  external_body=1  external_body_fn=1  loc=['Line 40 [cfg-verus]']
```

`external_body_fn` means "external_body NOT on an `external_type_specification`" — i.e. a **blocking** external_body on a user fn. But `ExFrameNumber` (phys.spec.rs:39–41) clearly *is* an `external_type_specification`:

```rust
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExFrameNumber(FrameNumber);
```

Root cause confirmed by minimal reproducer: the `#[cfg(verus_keep_ghost)]` sitting directly before `verus! {` perturbs the tree-sitter AST so `_sibling_has_attr()` no longer sees `external_type_specification` as a sibling of `external_body`:

```
verus!{ ExFrameNumber }                         -> external_body_fn=0  (correct: type-spec)
#[cfg(verus_keep_ghost)] verus!{ ExFrameNumber } -> external_body_fn=1  (WRONG: blocking)
```

By contrast `mm/phys/mod.spec.rs::ExLinkedList` (no cfg before its `verus!`) is correctly classified `external_type_spec`. So `ExFrameNumber` is being scored as a blocking external_body purely because of the redundant cfg gate.

(b) **Not listed in the TCB allow file.** `verus-ai-logs/tcb-allowed.md` documents `ExLinkedList` as an allowed `external_type_specification`, but `ExFrameNumber` (the opaque registration of the foreign `arch::FrameNumber`) is **absent**. Its sibling library-edge `assume_specification`s in `phys.spec.rs` (`::arch::mem::FRAME_SIZE`, `::arch::mem::FRAME_SHIFT`, `VirtualAddress::new`, `<VirtualAddress as Address>::into_raw_value`, `FrameNumber::into_raw_value`, `FrameNumber::from_raw_value`) are likewise undocumented, whereas tcb-allowed.md has an "Allowed `assume_specification`" section for the analogous `Address`-trait boundaries. Every surviving trust boundary must be enumerated.

### Fix Request

Two coupled changes — the first fixes #5 and the (a) half of #6/#9; the second fixes the (b) half.

1. **Remove the redundant cfg gate** in `src/kernel/src/hal/mem/types/address/phys.spec.rs`. Delete the `#[cfg(verus_keep_ghost)]` on line 21 so the block opens with a bare `verus! {` (matching `phys.proof.rs` and `mm/phys/mod.spec.rs`):

   ```rust
   // before
   #[cfg(verus_keep_ghost)]
   verus! {
   // after
   verus! {
   ```

   This is safe because the file is only `include!`d under `#[cfg(verus_keep_ghost)]` from `phys.rs:9`.

2. **Document the phys trust boundary** in `verus-ai-logs/tcb-allowed.md`: add `ExFrameNumber` as an allowed `external_type_specification` (foreign opaque `arch::FrameNumber`, mirroring `ExLinkedList`), and list the six `assume_specification` library-edge boundaries declared in `phys.spec.rs`.

**Verify after fixing — all must hold:**
```bash
# (a) cfg gate gone, ExFrameNumber reclassified as type-spec (eb_fn must be 0):
python3 -c "import sys; sys.path.insert(0,'/home/ruize/verus-ai-exp/verus-ai'); \
from pathlib import Path; from guardrails import detect_cheating; \
r=detect_cheating(Path('src/kernel/src/hal/mem/types/address/phys.spec.rs')); \
print('eb',r.external_body_count,'eb_fn',r.external_body_fn_count)"
#   expect: eb 1  eb_fn 0

# (b) module cfg-gate count for phys must drop to 0 (only include!/use remain, all ALLOWED)

# (c) regression — still verifies, still builds clean:
make verify-kernel      # exit 0, admit=0 assume=0 trusted=0 no_decreases=0
./z build -- all        # 0 errors, 0 warnings

# (d) no new spec drift / AST regression:
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/spec_drift.py \
  git-diff src/kernel/src/hal/mem/types/address/phys.rs --before 27abdb103171f505eea62d88c14f61886177d1d6
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py \
  src/kernel/src/hal/mem/types/address/phys.rs count   # still only the 2 pre-approved rewrites
```

Justification is not a fix: do not argue that `ExFrameNumber` is "obviously a type-spec" — change the cfg gate and show `eb_fn 0`, and add the entries to tcb-allowed.md.
