## Turn 1: Full checklist walkthrough — sys-virt-address cheating-elimination

### Progress
- Done (PASS/verified this turn): all 12 checklist items.
- Current: final sign-off.
- Remaining: none.

Target module: `src/libs/sys/src/sys/mm/address/virt.rs`
Spec/proof: `virt.spec.rs` / `virt.proof.rs` (both `verus! { }` — empty; specs are
inline `#[verus_spec]` on `new`/`from_raw_value`).
Verus run: `make verify-sys` → exit 0, 6 verified, 0 errors.

---

### Verification (per item)

**1. Zero admit() — PASS**
`make verify-sys` cheating scan: `admit=0`. Textual scan of `src/libs/sys/src/`
for `\badmit\s*\(` → NONE FOUND.

**2. Zero assume() — PASS**
Scan: `assume=0`. Textual scan → NONE FOUND.

**3. Zero trusted functions — PASS**
Scan: `trusted=0`.

**4. Zero exec_allows_no_decreases_clause — PASS**
Scan: `no_decreases=0`. Textual scan → NONE FOUND.

**5. Zero cfg-gated exec code — PASS (in scope)**
`grep verus_keep_ghost` in `virt.rs`: only lines 9 & 11, both gating
`include!("virt.spec.rs")` / `include!("virt.proof.rs")` — verification-material
**imports**, explicitly allowed by the rule ("only imports/derives/debug_assert/
logging allowed"). The detector exempts `include!` targets → virt.rs contributes 0.
The global `cfg_gate=1` for the sys crate originates at
`src/libs/sys/src/sys/mm/alignment.rs:151` (`#[cfg(verus_keep_ghost)] verus! { spec
fn spec_align_value ... }`). That is (a) **out of scope** — sys-virt-address target
functions live only in `virt.rs`; hard rule "do not touch unlisted functions" —
and (b) gating **ghost/spec** material (pure `spec fn`s), not exec code. The
heuristic flags it only because its exemption list covers `include!` but not an
inline `verus! {}` spec block. No exec code is cfg-gated in the target module.

**6. Zero external_body unless listed — PASS**
Scan: `external_body=0` for sys. Textual scan → NONE FOUND. Nothing to match against
`verus-ai-logs/tcb-allowed.md`.

**7. AST consistency: zero mismatches — PASS**
Tool: `scripts/ast_consistency.py`. Correct baseline is the pre-phase commit
`40a4c4b60` (the sys-virt-address phase START; `clone_address` and the spec
annotations of sibling modules already present). `summary` vs `40a4c4b60`:
matched=17, the only residual "MISMATCH" is `VirtualAddress::align_up`. Investigated:
`virt.rs` has **two** methods named `align_up` — inherent `align_up(&self) ->
Option<Self>` (line 90) and trait `impl Address::align_up(&self) -> Result<Self,
Error>` (line 199). Both exist identically in base `40a4c4b60` (lines 74 / 183) and
in HEAD; `git diff 40a4c4b60 HEAD -- virt.rs` shows **no change** to either body.
The name-based checker mis-pairs the inherent source method against the trait verus
method, producing a false-positive MISMATCH. Real exec is unchanged.
(Comparing against `dev`/merge-base additionally mis-flagged `from_raw_value` and
`clone_address`; both resolve to MATCH against the correct phase baseline —
`clone_address` was added in kernel-scope commit `40a4c4b60` as a now-required
`Address` trait method, not by this phase.)

**8. All exec rewrites have VERUS REWRITE comment + minimal reproducer — PASS (vacuous)**
`git diff 40a4c4b60..HEAD -- virt.rs` shows only additive annotations
(`#[verus_verify]`, `#[verus_spec]`, `use vstd::prelude::*`, the two `include!`s, the
`verus! { impl View }` block) and a cosmetic split of one `impl VirtualAddress` into
two impl blocks. No exec **body** was rewritten → no VERUS REWRITE comments required.
`grep "VERUS REWRITE"` → none (correct).

**9. Each surviving external_body listed in tcb-allowed — PASS (vacuous)**
No `external_body` survives in sys.

**10. No specs weakened — PASS**
`scripts/spec_drift.py check nanvix-phys-sys-virt-address` (baseline `b841740f9136`
→ HEAD): contract drift = 0, ensures removed = 0, requires added = 0.
"✅ No contract drift detected."

**11. Cross-module regression — PASS**
`make verify` (all crates): bitmap, sys, nanvix-slab, bump-allocator, arch, kernel
each **exit 0** (no verification errors). The `CHEATING_DETECTED` status lines on
other crates reflect their own in-progress verification debt (e.g. kernel
`admit=12 external_body=19`, bump-allocator `external_body=2`) and predate /
are independent of sys-virt-address — not regressions introduced here.

**12. Verification + build — PASS**
- `make verify-sys` → exit 0; 6 verified; `cheating: assume=0 external_body=0 admit=0
  trusted=0 no_decreases=0 cfg_gate=1` (cfg_gate sourced from out-of-scope
  alignment.rs ghost block, see item 5).
- `./z build -- all` → "Build complete", `Finished` with 0 compiler errors and
  0 warnings (the single `Warning: Sysroot directory ... not found; skipping symlink
  update` is a benign post-build symlink note, not a compilation warning).

### Fix Request
None. Every checklist item is PASS with concrete tool/code evidence. The lone
`cfg_gate=1` is an out-of-scope, ghost-only `verus!` spec block in `alignment.rs`
(hard rule forbids touching unlisted functions) and is not exec code; the single
residual AST "MISMATCH" is a name-collision false positive proven inert by
`git diff`. → RESOLVED.
