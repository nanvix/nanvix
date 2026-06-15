## Turn 1: Checklist walkthrough — FAIL on "No assume_specification for workspace-internal code"

### Progress
- Done (PASS / verified): items 1–6, 8–12, 14–17 (see Verification below).
- Current (FAIL, needs fix): item 7 — two new `assume_specification`s on
  workspace-internal types (`PhysicalAddress`, `PageAligned`) are not registered
  in `verus-ai-logs/tcb-allowed.md`.
- Flagged (deferred, acceptable for spec phase): item 13 — `admit=1` in
  `frame.proof.rs` (`lemma_phys_view_is_spec_addr`), the bridge lemma whose body
  is the proving-phase obligation.
- Remaining: re-verify item 7 after fix.

### Verification (commands run, evidence)

Tooling state:
- `make verify-kernel` → exit 0. `note: verifying module hal::mem::types::address::frame`.
  Cheating check: `assume=0 external_body=24 admit=1 trusted=0`. Coverage 51/1022.
- `./z build -- all-kernel` → exit 0 (`[OK] Build complete`, `kernel.elf` produced).
- Spec drift: `git diff 9d2f5c9 -- frame.rs frame.spec.rs frame.proof.rs`.

Per-item findings (in-scope = type `FrameAddress`, `from_frame_number`,
`into_frame_number`, `from_raw_value`, `into_raw_value`; per caller_analysis.md
`new`/`into_physical_address`/`into_page_address`/`Debug`/`PartialEq` are out of
scope):

1. **Every in-scope exec fn has requires/ensures** — PASS.
   `from_frame_number` (ensures `result is Ok` + `Ok(fa) ==> fa@ == spec_from_number(spec_frame_raw_value(frame_number)) && fa.inv()`),
   `into_frame_number` (requires `self.inv()`, ensures `spec_frame_raw_value(result) == spec_frame_number(self@)`),
   `from_raw_value` (ensures `Ok(fa) ==> fa@ == raw_addr as int && fa.inv()`),
   `into_raw_value` (ensures `result as int == self@`). None of these appear in
   `coverage-unverified.txt`.

2. **Caller coverage** — PASS. Checked each expectation in `caller_analysis.md`:
   - `from_frame_number`: caller expects `fa@ == frame_number*PAGE_SIZE && fa.inv()`,
     Err value-free. Spec proves `result is Ok` (stronger: never fails) and
     `fa@ == spec_from_number(spec_frame_raw_value(frame_number))`. Confirmed
     `spec_from_number(n) == n * spec_page_size()` (phys.spec.rs:70). ✓
   - `into_frame_number`: caller expects total, `result == self@/PAGE_SIZE`,
     bounded. Spec: `spec_frame_raw_value(result) == spec_frame_number(self@)`,
     `spec_frame_number(addr) == addr/spec_page_size()` (phys.spec.rs:65); the
     `requires self.inv()` is satisfied by every constructor's `ensures fa.inv()`
     and by `mm::phys` consumers (verification passes crate-wide). ✓
   - `from_raw_value`: caller (`boot_init.rs:207`) expects `fa@ == raw_addr && fa.inv()`
     on Ok, propagate on Err. Spec matches. ✓
   - `into_raw_value`: caller expects `result as int == self@`. Spec matches. ✓

3. **View consistency** — PASS. `view_design.md`: `type V = int`, `view = self.0@`
   (closed), `inv = self@ % spec_page_size() == 0 && spec_frame_number(self@) <= spec_max_frame_number()`.
   Every contract references `self@` / `fa@` / `inv()`; constructors establish
   `inv()`. Consistent with the confirmed view-design.

4. **No tautological ensures** — PASS. No in-scope exec `ensures` is `=> true`.
   (The `Err(_) => true` arm lives inside the `<PhysicalAddress as Address>::from_raw_value`
   `assume_specification`, a trust boundary; it is justified — see item 6 — and
   matches the caller-analysis "Error path is value-free".)

5. **No subsumed ensures** — PASS. `from_frame_number`'s `result is Ok` is an
   independent (stronger) guarantee, not derivable from the `Ok(..)` arm.
   `fa.inv()` is not derivable from the address-identity ensures alone.

6. **Error paths meaningful** — PASS. `from_frame_number` proves `result is Ok`
   (eliminates the error path entirely — strongest form). `from_raw_value` leaves
   Err unconstrained: this is correct, not lazy — the failure can arise from
   either the physical-range check or the alignment check, so the biconditional
   sketched in view_design.md (`Err <==> raw % page_size != 0`) is *not* provable;
   caller_analysis.md confirms the sole caller only branches Ok/Err.

7. **No assume_specification for workspace-internal code** — **FAIL**. See Fix Request.

8. **vstd searched before assume_specification** — PASS. The three boundaries are
   `::arch::mem::PAGE_SIZE` (arch crate const), `<PhysicalAddress as Address>::from_raw_value`
   (`sys::mm::Address` trait method), and `<PageAligned<T> as core::ops::Deref>::deref`
   (core trait method). None have a vstd-provided spec; inline rationale documents
   why each is a library-edge boundary.

9. **Specs written for the caller** — PASS. All stated over the `int` View
   (`self@`) and `inv()`, never over the inner `PageAligned<PhysicalAddress>`.

10. **Trait obligations** — PASS. `Debug`/`PartialEq` are out of scope and
    untouched.

11. **Spec completeness (advisory)** — PASS (advisory). Only intentional
    nondeterminism is the value-free Err on `from_raw_value`, which matches the
    documented caller expectation.

12. **Loop invariants** — PASS (N/A). No loops in `frame.rs`.

13. **No cheating on module's own functions** — Report: in `frame.rs`/`frame.spec.rs`/`frame.proof.rs`:
    `assume=0`, `external_body=0` (note: `into_raw_value` was de-trusted — the
    prior `#[verus_verify(external_body)]` is gone and it is now body-verified
    against `self.0.into_raw_value()`), `trusted=0`, `admit=1`. The single `admit`
    is `lemma_phys_view_is_spec_addr` (`frame.proof.rs:32`), the bridge between
    `spec_addr(&pa)` and `pa@`. **Deferred / acceptable for the specification
    phase** (it is a pure proof lemma, no exec content; its discharge is the
    proving-phase obligation, as the file header states). Must be discharged
    (admit removed) in the proving phase — flagged, not a spec-phase blocker.

14. **No specs weakened** — PASS. Drift diff vs `9d2f5c9` shows only
    strengthening: `inv()` gained the conjunct
    `spec_frame_number(self@) <= spec_max_frame_number()`; `into_raw_value` lost
    `external_body` (now verified, same ensures); `from_frame_number` /
    `into_frame_number` / `from_raw_value` gained contracts where they had none.
    Confirmed `<PhysicalAddress as Address>::from_raw_value` has no competing
    verified spec elsewhere (phys.rs:230 is the only def, unspecced), so the new
    `assume_specification` does not shadow/weaken anything.

15. **Bug awareness** — PASS. No fundamentally incorrect code found. The bridge
    lemma's claim (`spec_addr(&pa) == pa@`) is semantically sound. No bugs file
    exists and none needs creating.

16. **Cross-module regression** — PASS. `make verify-kernel` verifies the whole
    `kernel` crate (cheating check spans all 24 module `external_body`s); exit 0.

17. **Verification + build** — PASS. `make verify-kernel` exit 0, 0 errors;
    `./z build -- all-kernel` exit 0.

### Fix Request (item 7)

`frame.spec.rs` introduces **two new `assume_specification`s on workspace-internal
types** that are **not registered** in `verus-ai-logs/tcb-allowed.md`:

- `frame.spec.rs:110` — `pub assume_specification[ <PhysicalAddress as Address>::from_raw_value ]`
  (`PhysicalAddress` is kernel-internal; defined at `phys.rs:230`, unspecced).
- `frame.spec.rs:129` — `pub assume_specification<T: Address>[ <PageAligned<T> as core::ops::Deref>::deref ]`
  (`PageAligned` is kernel-internal).

The project's TCB discipline requires every library-edge `assume_specification`
to be registered in `tcb-allowed.md` with a dedicated rationale section (see the
existing sections for `kframe.spec.rs`, `page.spec.rs`, `phys.spec.rs` at lines
181, 217, 252). These two frame-module boundaries have no entry — a `grep -n -i
'frame\.spec\|::deref\|<PhysicalAddress' verus-ai-logs/tcb-allowed.md` returns no
frame.spec.rs registration.

**Do this:** Add a new section to `verus-ai-logs/tcb-allowed.md`, e.g.
`## Allowed assume_specification — hal::mem::types::address::frame library edge`,
that registers BOTH declarations by fully-qualified path:

- `<crate::hal::mem::types::address::PhysicalAddress as sys::mm::Address>::from_raw_value`
  (declared in `src/kernel/src/hal/mem/types/address/frame.spec.rs`) — state its
  ensures (`Ok(r) => r@ == value && spec_frame_number(r@) <= spec_max_frame_number()`;
  `Err(_) => true`) and the justification already in the inline comment: the
  `impl Address for PhysicalAddress` cannot be body-verified in place because its
  sibling `usize as *const u8` casts are Verus-unsupported (cite
  `verus-ai-logs/verus-unsupported.md`); precedent is the `sys::mm::Address`
  boundary at `kframe.spec.rs` / `page.spec.rs`. Note "Removed when `hal::mem` is
  verified."
- `<crate::hal::mem::PageAligned<T> as core::ops::Deref>::deref`
  (declared in the same file) — pure-projection ensures
  (`spec_addr(result) == addr@`), precedent `page.spec.rs`.

This is a documentation/registration change only — do **not** weaken or remove
the contracts. After editing, verify nothing regressed:

```
make verify-kernel          # expect exit 0, assume=0, no new admit/external_body
grep -n -i "frame.spec\|PhysicalAddress as\|::deref" verus-ai-logs/tcb-allowed.md
```

The second command must now show the two new entries.

(If you instead believe these should NOT be assume_specification — i.e. they can
be body-verified or already have a verified spec in `phys`/`page` — show the
tool output proving it: a verified `#[verus_spec]` on
`<PhysicalAddress as Address>::from_raw_value` and a `make verify-kernel` exit 0
without the `frame.spec.rs` declarations. Justification text alone is not a fix.)
