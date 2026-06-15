## Turn 1: Full Checklist Walk — sys-virt-address (Proving)

Source : `src/libs/sys/src/sys/mm/address/virt.rs`
Spec   : `src/libs/sys/src/sys/mm/address/virt.spec.rs`  (`verus! { }` — empty)
Proof  : `src/libs/sys/src/sys/mm/address/virt.proof.rs` (`verus! { }` — empty)
Proving start SHA: `c738ce2c19567660c01b06ab7fa15a7b427abd12`
Verus  : `make verify-sys` → **6 verified, 0 errors** (exit 0)
Build  : guest non-verus `cargo build` → **Finished, 0 warnings, 0 errors**

### Progress
- Done (PASS): all 10 checklist items (see below).
- Current: none — verdict RESOLVED.
- Remaining: none.

---

### Item-by-item verification

#### 1. No specs weakened — **PASS**
- `git diff c738ce2c..HEAD -- virt.rs virt.spec.rs virt.proof.rs` → **empty** (proving phase made zero source changes).
- `spec_drift.py git-diff src/libs/sys/src/sys/mm/address/virt.rs --before c738ce2c` → exit 0, "✅ No contract drift detected" (0 ensures removed, 0 requires added).
- Surviving specs are intact and strong:
  - `VirtualAddress::new` → `ensures result@ == value as int`
  - inherent `from_raw_value` → `ensures result@ == raw_addr as int`
  - `impl View for VirtualAddress` (`view == self.0 as int`)
  - `<… as Address>::into_raw_value` contract lives at the trait decl in `mod.rs` (`ensures result as int == self@`), binding all implementers.

#### 2. Zero remaining admit() — **PASS**
- Global cheating scan: `admit=0`.
- `grep` of virt.rs/spec.rs/proof.rs: no `admit`.

#### 3. Zero external_body unless TCB-listed — **PASS**
- Global scan: `external_body=0`. `tcb_entry_points: null` in pipeline_state.json.
- No `#[verifier::external_body]` in any virt file.

#### 4. Zero assume / assume_specification — **PASS**
- Global scan: `assume=0`. No `assume` / `assume_specification` in virt files.

#### 5. No cfg-gated exec code — **PASS (for the proving target)**
- virt.rs `#[cfg(...)]` usages:
  - L9/L11 `#[cfg(verus_keep_ghost)] include!("virt.spec.rs"/"virt.proof.rs")` — gate spec/proof includes only (standard pattern, excluded by detector).
  - L39/L296 `#[cfg(target_pointer_width = "32")]` — arch gating of a `static_assert!` macro and a `From` impl; **not** verus ghost-gating, not exec branches/expr/match-arms.
- Zero cfg-gated **exec** branches/expressions/match arms in virt.rs.

#### 6. Cheating audit — counts & locations — **PASS (no exec-code violation)**
Exact crate-wide counts from `make verify-sys`:
- `admit=0`, `external_body=0`, `assume=0`, `trusted=0`, `no_decreases=0`
- `cfg_gate=1` → **single location: `src/libs/sys/src/sys/mm/alignment.rs:151`**

Classification of the lone `cfg_gate=1`:
- It is `#[cfg(verus_keep_ghost)]` on a `verus! { … }` block whose *only* contents are `pub open spec fn spec_align_value(...)` (lines 151–182). This gates **ghost/spec code, not exec code** — it is therefore **not** a violation of the "no cfg-gated exec code" rule (the heuristic over-counts inline `verus!{}` blocks).
- It is **out of scope** for sys-virt-address: it lives in the `alignment` module (separate proof target), and is **pre-existing** (recorded as `sys::all … cfg_gate=1` in commits `cf8e3a993` and earlier, before this proving phase started).
- Per repository rule "Don't fix pre-existing issues unrelated to your task," no change to `alignment.rs` is requested in this virt-address review. (Note for the alignment target: virt.rs's own `verus!{}` block at L319 is ungated — the redundant gate at `alignment.rs:151` should be dropped there to clear the heuristic.)

#### 7. Claimed Verus limitation has an isolated reproducer — **PASS**
- `verus-unsupported.md` documents the real front-end limitation with the exact, isolated construct:
  - `self.0 as *const u8` / `as *mut u8` → "Verus does not support this cast: `usize` to `*const u8`" (virt.rs:268/272).
- Secondary cascade (all-or-nothing trait verification pulling in out-of-scope inherent/`mm::*` methods) is documented with exact errors and the in-scope resolution (contract attached at the trait declaration in `mod.rs`, impl left unannotated so the crate stays at 0 errors). Acceptable.

#### 8. Exec rewrites minimal & semantically equivalent — **PASS**
- Zero exec changes during proving (diff empty). No `// VERUS REWRITE` comments present or needed.

#### 9. Cross-module regression — **PASS**
- `make verify-sys` runs `<all>` modules of the sys crate → **6 verified, 0 errors**. All verified modules pass.

#### 10. Verification + build, 0 errors / 0 warnings — **PASS**
- Verify: `6 verified, 0 errors` (exit 0).
- Build: guest target non-verus `cargo build --locked -Zbuild-std=core,alloc …` → `Finished … target(s)`, **no warnings, no errors**.

---

### Fix Request
None. All checklist items PASS with tool-backed evidence. The only flagged
cheating pattern (`cfg_gate=1`) is a heuristic false positive on a spec-only
`verus!{}` block in `alignment.rs:151` — ghost code, pre-existing, and outside
this proving target's scope. No code change is required for sys-virt-address.

### Verdict: RESOLVED
