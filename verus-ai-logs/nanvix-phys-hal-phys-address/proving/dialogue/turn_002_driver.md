## Turn 2: Item 4 (assume/assume_specification) — verifying the fix; final sign-off

### Progress
- Done (PASS/FIXED), all verified independently this turn:
  1. **No specs weakened** — PASS (turn 1: spec-phase→now diff only *strengthened*
     specs; 4 assume_specs removed, `uninterp`→concrete `open`; per-fn contracts
     byte-identical).
  2. **Zero admit()** — PASS (`grep admit phys.{rs,spec.rs,proof.rs}` = empty; all
     27 admits in out-of-scope `mm/phys/*`,`mm/virt/*`).
  3. **Zero external_body** — PASS (the only `external_body` token in phys files is
     inside a doc-comment, `phys.spec.rs:69`; cheating-detail lists none for this
     module).
  4. **Zero assume/assume_specification** — **FIXED** (see Verification).
  5. **No cfg-gated exec code** — PASS (only `#[cfg(verus_keep_ghost)]` on the two
     `include!` lines, excluded by the counter).
  6. **Cheating audit** — PASS. phys module: admit=0, external_body=0,
     assume_specification=1 (documented, see #4), cfg-gated exec=0.
  7. **Claimed limitation has isolated reproducer** — **FIXED** (see Verification).
  8. **Exec rewrites minimal/equivalent** — PASS (`from_number`/`into_frame_number`
     let-bindings are semantically identical; no `// VERUS REWRITE` needed).
  9. **Cross-module regression** — PASS (`make verify-kernel` exit 0, all modules;
     `make verify-sys` CLEAN).
  10. **Verification + build 0/0** — PASS (`make verify-kernel` exit 0, phys
      `6 verified, 0 errors`; `./z build -- all` ⇒ `[OK] Build complete.`, no rustc
      warnings/errors).

- Current: Item 4 + Item 7 (the only open FAIL/CONCERN from turn 1).
- Remaining: none — all 10 items PASS/FIXED.

### Verification

**Item 4 — `assume_specification[ <VirtualAddress as Address>::into_raw_value ]`
(phys.spec.rs:74-79).** Turn 1 demanded: eliminate it by speccing the real `sys`
method, **or** (only if a genuine Verus limitation blocks that) provide an isolated
reproducer **and** a `tcb-allowed.md` entry. The fixer took the escape-hatch branch.
I verified every claim rather than trusting the writeup:

- **The limitation is real — confirmed by running the reproducers through the actual
  Verus binary** (`/home/ruize/toolchain/verus/verus`):
  - `specification/whole_impl_rule.rs` →
    `error: In order to verify any items of this trait impl, the entire impl must be
    verified.` — verbatim match.
  - `specification/ptr_cast.rs` →
    `error: Verus does not support this cast: `usize` to `*const u8`` — verbatim
    match.
  These are minimal, isolated snippets of the two specific constructs (trait-impl
  whole-impl rule; int-to-ptr cast in the sibling `as_ptr`/`as_mut_ptr`), not the
  full failing expression. The net-trust argument is sound: verifying the impl would
  require **two** `external_body` (on `as_ptr`/`as_mut_ptr`, genuine int-to-ptr
  bottoms) to remove **one** trivial getter assumption — a net expansion of the TCB.
- **The `sys` regression was genuinely fixed.** `git log` on
  `src/libs/sys/src/sys/mm/address/virt.rs`: `c7a556350` = `verify FAIL: sys::all`,
  `24a56f3f0` = `verify PASS: sys::all (6 verified, 0 errors)`. Current
  `impl Address for VirtualAddress` (virt.rs:176) carries **no** `#[verus_verify]`
  (line 46 `#[verus_verify]` is on the *inherent* impl, not this trait impl). I ran
  `make verify-sys` myself: **exit 0, CLEAN, assume=0 external_body=0 admit=0**.
- **`tcb-allowed.md` entry present and complete** (lines 170-195): function path,
  `ensures result as int == addr@`, both verbatim error messages, the empirical
  regression evidence, and the net-trust rationale.
- **Contract unchanged** — still `ensures result as int == addr@`; no spec weakened.
- I re-ran `make verify-kernel`: exit 0; module `hal::mem::types::address::phys`
  `6 verified, 0 errors`; cheating-detail.txt contains **no** entry for
  `types/address/phys`. Global `assume=0` (the scan's `assume` counter does not
  include `assume_specification`; the single retained one is the documented
  boundary).

→ Item 4 **FIXED** via the explicitly-sanctioned escape hatch, with all conditions
met and independently re-verified.

**Item 7 — isolated reproducers.** The cheating-relevant limitation
(`into_raw_value`) now has two isolated reproducers, both of which I executed and
confirmed reproduce the claimed errors verbatim. The `phys.rs:143-148` `VERUS
DEVIATION` note about cross-crate `use_type_invariant` justifies an equivalent
`let`-binding rewrite (item 8 territory), not a cheating construct — the proof
discharges its bound from `into_raw_value`'s real method contract, and the rewrite
is semantically identical ("Evaluation order and effects are identical"). Acceptable;
no further reproducer required. → Item 7 **FIXED/PASS**.

### Fix Request
None. All checklist items are PASS or FIXED with independently re-verified evidence:
- `make verify-sys` → exit 0, CLEAN.
- `make verify-kernel` → exit 0, `phys` 6 verified / 0 errors, module clean.
- `./z build -- all` → `[OK] Build complete.`, no rustc warnings/errors.
- Both Verus reproducers executed and reproduce the documented errors verbatim.
- `tcb-allowed.md` entry complete for the single, genuinely-irreducible
  `assume_specification`.

Writing `STOP = RESOLVED`.
