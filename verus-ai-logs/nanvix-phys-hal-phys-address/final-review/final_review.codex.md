# Final Verification Review — `hal::mem::types::address::phys`

Date: 2026-06-15
Branch: `verus-ai-prove`
Reviewer mode: independent, strict, tool-verified.

## Scope checked
In-scope only:
- `PhysicalAddress` (type / `View` / `inv`)
- `PhysicalAddress::from_number`
- `PhysicalAddress::into_frame_number`
- `PhysicalAddress::from_mmio_address`

Files:
- `src/kernel/src/hal/mem/types/address/phys.rs`
- `src/kernel/src/hal/mem/types/address/phys.spec.rs`
- `src/kernel/src/hal/mem/types/address/phys.proof.rs`

---

## 1) Spec quality (requires/ensures)

### Findings
- `from_number` contract (`phys.rs:138-141`) is concise and caller-useful:
  - `ensures result@ == spec_from_number(spec_frame_raw_value(frame))`
- `into_frame_number` contract (`phys.rs:160-165`) captures the key semantic projection:
  - `requires self.inv()`
  - `ensures spec_frame_raw_value(result) == spec_frame_number(self@)`
- `from_mmio_address` contract (`phys.rs:112-119`) captures identity + total-Ok behavior under precondition:
  - `requires spec_frame_number(addr@) <= spec_max_frame_number()`
  - `ensures result is Ok`, `Ok@ == addr@`, `Ok.inv()`

### Quality notes
- No tautological ensures found.
- One subsumption/redundancy: in `from_mmio_address`, `Ok.inv()` is derivable from `requires` + `Ok@ == addr@` + `inv` definition (`phys.spec.rs:43-45`). Not unsound; just redundant.
- Error-path spec: function explicitly specifies `result is Ok`, so Err path is intentionally excluded (strong, meaningful, and consistent with body).

---

## 2) Caller coverage vs `caller_analysis.md`

Mapped caller assumptions to contracts/provable consequences.

- **Covered directly**: totality of `from_number`, base-address relation, `into_frame_number` frame-index equality, `from_mmio_address` identity-on-Ok, Err-unreachable behavior.
- **Covered by derivation**: frame alignment from `from_number`; same-frame/different-frame behavior from division semantics; round-trip intent from combined specs plus arithmetic facts.
- **Partially/not encoded as requires/ensures in-scope**:
  - generic-wrapper usability (`PageAligned`/`TruncatedMemoryRegion`/trait-level behavior) is type/trait-level and outside these three function contracts.

**Coverage summary: 13 / 14 caller expectations covered; 1 not directly encoded in in-scope contracts.**

---

## 3) Proof completeness (`admit`, `external_body`) in phys trio

Tool scan on:
- `phys.rs`
- `phys.spec.rs`
- `phys.proof.rs`

Results:
- `admit()` = **0**
- `external_body` = **0**

Tool output:
```text
rg "admit\(" ... => No matches found.
rg "external_body" ... => No matches found.
```

Status: passes this dimension.

---

## 4) TCB compliance (`external_body` in-scope)

In-scope `external_body` occurrences: **none**.
Therefore TCB allowed-list check for in-scope `external_body` is vacuously satisfied.

Also checked `tcb-allowed.md` for `VirtualAddress::into_raw_value`; no explicit entry found.

Tool output:
```text
rg "VirtualAddress.*into_raw_value|into_raw_value.*VirtualAddress|sys::mm::VirtualAddress" verus-ai-logs/tcb-allowed.md
No matches found.
```

---

## 5) AST consistency + `// VERUS REWRITE` audit

Command run:
```text
python3 /home/ruize/verus-ai-exp/verus-ai/scripts/ast_consistency.py --base-ref verus-ai-prove src/kernel/src/hal/mem/types/address/phys.rs summary
```

Output (actual):
```text
Consistent: ✅ YES (matched=17 mismatched=0 missing=0 extra=0)
```

### Rewrite comment audit
- `phys.rs:142` (`from_number` rewrite): semantic equivalence holds (same `into_raw_value` value multiplied by `FRAME_SIZE`; split into intermediate binding + proof call only).
- `phys.rs:277` (`clone_address` rewrite): method addition tied to trait contract evolution; body `PhysicalAddress(self.0)` is view-preserving clone (`result@ == self@`). No semantic mismatch found.

Verdict: no AST mismatch blocker.

---

## 6) Verification run (`make verify-kernel`)

Command run (repo root):
```text
make verify-kernel
```

Actual output (key lines):
```text
error: cannot use function `sys::sys::mm::address::virt::impl&%1::into_raw_value` which is ignored because it is either declared outside the verus! macro or it is marked as `external`.
   --> src/kernel/src/hal/mem/types/address/phys.rs:167:31
    = help: The following declaration may resolve this error:
            pub assume_specification [<sys::mm::VirtualAddress as sys::mm::Address>::into_raw_value] (_0: sys::mm::VirtualAddress) -> usize;
...
Exit code : 101
make: *** [Makefile:625: verify-kernel] Error 101
```

Result: **FAIL** (required exit 0 not met).

---

## 7) Guardrails (exact counts + locations for phys module)

Scanned `phys.rs`, `phys.spec.rs`, `phys.proof.rs`.

- `admit()` = **0** (no locations)
- `assume()` = **0** (no locations)
- `external_body` = **0** (no locations)
- active `assume_specification` declarations = **0** (no active location)
- cfg-gated exec items = **0**

Related non-exec cfg locations:
- `phys.rs:9` `#[cfg(verus_keep_ghost)]`
- `phys.rs:11` `#[cfg(verus_keep_ghost)]`

Important context in current tree:
- Commented-out assumption scaffold exists at:
  - `phys.spec.rs:61` `// EXPERIMENT: temporarily commented out for review`
  - `phys.spec.rs:62-67` commented `assume_specification` lines.

Tool outputs:
```text
rg "admit\(" ... => No matches found.
rg "assume\(" ... => No matches found.
rg "external_body" ... => No matches found.
rg "^\s*pub\s+assume_specification\s*\[" phys.spec.rs => No matches found.
rg "#\[cfg\(" ...
  phys.rs:9
  phys.rs:11
```

---

## 8) Bug reconciliation (`bugs.md`)

`bugs.md` states "None".

Current surviving issue is not an exec-code bug; it is a verification/spec-integration regression:
- commented-out `assume_specification` for `VirtualAddress::into_raw_value` causes verification failure.

Classification per bug-reporting skill: **False Positive / proof-spec infrastructure issue** (not a True Bug in exec semantics).

---

## SPECIAL INVESTIGATION — `VirtualAddress::into_raw_value` assume_specification

Target item (prompted):
- `phys.spec.rs` around line 61 (`assume_specification` for `<VirtualAddress as Address>::into_raw_value`)

Current state: declaration is commented out; verification fails.

Evidence:
- Module verify failure (`make verify-kernel MODULE=hal::mem::types::address::phys`) reports missing declaration and points to exact suggested `assume_specification`.
- `Address` trait in `sys` has a verified trait-level spec (`src/libs/sys/src/sys/mm/address/mod.rs:63-67`).
- But concrete `impl Address for VirtualAddress` (`src/libs/sys/src/sys/mm/address/virt.rs:167+`) is **not** marked `#[verus_verify]`; Verus treats the method as ignored for this call site, so trait-level spec is insufficient here.

Definitive conclusion:
- The assumption is **currently required** to verify this module (cross-crate/ignored-impl gap in current verification setup).
- It is **not** a removable redundant assume in the current state.
- However, it also conflicts with checklist goals "no workspace-internal assume_specification" / "zero assume_specification". So this is a policy-vs-compilability conflict, unresolved.

---

## Prioritized issues

1. **BLOCKER** — `make verify-kernel` fails (exit 101) due missing active spec for `VirtualAddress::into_raw_value` at `phys.rs:167`.
2. **BLOCKER (checklist compliance)** — Current state cannot satisfy both:
   - zero workspace-internal `assume_specification`, and
   - successful verification of `phys` using current `sys::VirtualAddress` impl visibility.
3. **Minor quality** — `from_mmio_address` postcondition includes a redundant `inv()` ensure (derivable).

---

## Final verdict: **FAIL**

Reason: at least one blocker is present.
- Verification requirement not met (`make verify-kernel` exit 0 expected, observed exit 101).
- Special-investigation checklist goals are unmet in current state (assumption removed => break; assumption present => workspace-internal trust assumption).

