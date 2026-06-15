## Turn 2: Item #4 fix verification — all items now PASS/FIXED

### Progress
- Done (PASS turn 1): #1 spec-drift, #2 admit, #3 external_body, #5 cfg-gated
  exec, #6 cheating audit, #7 reproducer, #8 exec rewrites, #9 cross-module, #10 verify.
- FIXED this turn: **#4 Zero assume/assume_specification.**
- Remaining: none — entire checklist verified.

### Verification

Fixer applied **Option B** (TCB-sanctioned). I verified every claim independently.

**1. `assume_specification` removed — confirmed.**
`grep -rn 'assume_specification\|assume(' src/kernel/src/hal/mem/types/address/frame*.rs`
returns only two hits, both **comment prose** (frame.rs:40, frame.spec.rs:11).
The declaration block (former frame.spec.rs:20-27) is gone; `frame.spec.rs` now
holds only the `use` import (still needed by `from_frame_number`/`into_frame_number`
ensures + `frame.proof.rs`) and a removal-rationale comment.

**2. Trust relocated to a TCB-listed boundary — confirmed.**
`FrameAddress::from_raw_value` (frame.rs:94) now carries `#[verus_verify(external_body)]`.
This exact function is listed in `tcb-allowed.md:137-138`
(`...frame.rs::FrameAddress::from_raw_value — succeeds only for page-aligned
inputs, so ensures Ok(fa) => fa.inv()`). So it is **not** an unsanctioned hole.
`cheating-detail.txt` confirms the only frame entry is
`hal/mem/types/address/frame.rs:102 from_raw_value: external_body` — the single
TCB-approved function. No other frame function is external_body/admit/assume.

**3. Contract preserved verbatim (no weakening) — confirmed.**
The `#[verus_spec]` ensures `Ok(fa) => fa.inv() && fa@ == raw_addr as int` is kept
byte-for-byte; only the discharge moved from in-body proof to the trusted boundary.
`spec_drift.py git-diff --before 1f6205c56` on all three frame files →
**✅ No contract drift** (ensures removed: 0, requires added: 0, functions
removed: 0). Removing the intra-crate `assume_specification` is not flagged as a
weakening because no caller-facing contract was relaxed.

**4. Verification (non-cached, full kernel) — I ran `make verify-kernel` myself:**
`96 verified, 0 errors, exit 0`. The `^^^^^^` lines in the log are
trigger-selection `note:` messages in `mm/phys/frame.proof.rs` (another module),
**not warnings** — `grep -i warning` on the log returns nothing.
Cheating counts: `assume=0 external_body=20 admit=12`. The external_body 19→20
delta is exactly this one TCB-listed function; admit unchanged (all 12 in other
modules). Re-audit of items #2/#3/#5/#6/#9 against the fresh run: still PASS.

**5. Build — I ran `./z build -- all-kernel` myself:**
`Finished dev profile ... Build complete.`, exit 0, no errors, no warnings.

### Fix Request
None. Item #4 is FIXED with tool-verified evidence:
- intra-crate `assume_specification` deleted,
- trust carried by the TCB-listed `external_body` on `FrameAddress::from_raw_value`,
- contract preserved verbatim (0 spec drift),
- `make verify-kernel` 96 verified / 0 errors / 0 warnings,
- `./z build -- all-kernel` clean.

All 10 checklist items are PASS/FIXED. → RESOLVED.
