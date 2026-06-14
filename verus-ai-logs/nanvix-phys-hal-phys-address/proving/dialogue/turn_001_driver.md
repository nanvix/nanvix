## Turn 1: Item 4 — Zero assume/assume_specification

### Progress
- Done (PASS):
  - **No specs weakened** — `git diff f81c68273(spec-phase end) HEAD` on `phys.spec.rs`/`phys.rs`:
    specs were *strengthened*, never weakened. `spec_frame_raw_value` and
    `spec_max_frame_number` went from `uninterp` → concrete `open spec fn`
    (`frame@`, `FrameNumber::spec_max() as int`), and **4** placeholder
    `assume_specification`s were removed (VirtualAddress::new, FRAME_SHIFT,
    FrameNumber::from_raw_value/into_raw_value). Per-function `#[verus_spec]`
    contracts (`from_mmio_address`, `from_number`, `into_frame_number`, `View`)
    are byte-identical to the captured baseline `body_removed_source.rs` — diff
    shows only added (`+`) contract lines, zero removed (`-`) contract lines.
  - **Zero admit()** — `cheating-detail.txt` lists 27 admits, all in `mm/phys/*` /
    `mm/virt/*` (out of scope). `grep admit( phys.rs phys.spec.rs phys.proof.rs` = empty.
  - **Zero external_body** — 11 external_body all in `mm/phys/*`; none in the
    `hal/mem/types/address/phys` module. Verified against `tcb-allowed.md`.
  - **No cfg-gated exec code** — only `#[cfg(verus_keep_ghost)]` on the two
    `include!("phys.spec.rs"/"phys.proof.rs")` lines; the cfg-gate counter in
    `verify.sh` explicitly excludes `include!`. The reported `cfg_gate=14` is all
    elsewhere in the kernel (out of scope).
  - **Cheating audit (phys module)** — admit=0, external_body=0,
    assume_specification=1, cfg-gated exec=0.
  - **Exec rewrites minimal/equivalent** — `from_number` binds
    `let raw_value = frame.into_raw_value();` before the multiply, and
    `into_frame_number` binds `let shift = mem::FRAME_SHIFT;`. Both are
    semantically identical to the inline forms; no `// VERUS REWRITE` markers
    needed.
  - **Cross-module regression** — `make verify-kernel` exit 0; module
    `hal::mem::types::address::phys` verified (6 verified, 0 errors) and all other
    verified modules still pass.

- Current: **Item 4 — Zero assume/assume_specification (FAIL)**.

- Remaining to fully sign off:
  - Item 7 (claimed Verus limitation has isolated reproducer) — CONCERN, see below.
  - Item 10 (`make verify-kernel` + build: 0 errors, 0 warnings) — verify-kernel
    is exit 0; the build half still needs to be run and shown clean.

### Verification
`phys.spec.rs:61-66` still contains:

```rust
pub assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ](
    addr: VirtualAddress,
) -> (result: usize)
    ensures
        result as int == addr@,
;
```

This is consumed by verified bodies: `phys.rs:166`
(`let raw_addr: usize = self.0.into_raw_value();` in `into_frame_number`) and
`phys.rs:275` (`into_raw_value`).

The checklist allows `assume_specification` **only** for external-bottom trust
boundaries on std/genuinely-unverifiable crates. `sys` is a first-party workspace
crate that is *in scope* for this bottom-up effort (`sys-virt-address` /
`nanvix-phys-sys-virt-address` logs exist), so it is not a genuine bottom.

Decisive evidence this is removable, not a bottom: the **sibling** placeholder for
`VirtualAddress::new` in this same file was already eliminated this phase by giving
`sys` a real `#[verus_spec]` (`virt.rs:48-52`,
`#[verus_spec(result => ensures result@ == value as int)]`). The body of
`into_raw_value` is trivially `self.0` (`virt.rs:253-255`) and the `View` is
`closed spec fn view = self.0 as int` (`virt.rs:321-327`), so the exact same
`result as int == self@` postcondition is provable in `sys`.

The one nuance: `new` is an **inherent** method (easy to annotate), whereas
`into_raw_value` is a **trait** method (`Address::into_raw_value`) — the
`impl Address for VirtualAddress` block (`virt.rs:167`) carries no `#[verus_verify]`
and the method no `#[verus_spec]`. That is a *reason it was skipped*, not a proof
that it is irreducible. It is also **undocumented** in `verus-ai-logs/tcb-allowed.md`.

### Fix Request
Eliminate the placeholder — do exactly what was done for `VirtualAddress::new`:

1. In `src/libs/sys/src/sys/mm/address/virt.rs`, give the real
   `Address::into_raw_value` a verified spec so its postcondition is
   `result as int == self@` (annotate the `impl Address for VirtualAddress` block
   `#[verus_verify]` and add
   `#[verus_spec(result => ensures result as int == self@)]` to `into_raw_value`,
   or whatever mechanism `sys` already uses to spec trait-impl methods).
2. Delete the `assume_specification[ <::sys::mm::VirtualAddress as ::sys::mm::Address>::into_raw_value ]`
   block from `phys.spec.rs` (lines 61-66), replacing it with the same "real
   specification supersedes it" note used for `new`.
3. Verify: run `make verify-sys` (sys must still verify with the new spec) **and**
   `make verify-kernel` — both must be exit 0 with `phys` still
   `6 verified, 0 errors` and the global cheating line still `assume=0`.

If — and only if — a *genuine* Verus limitation makes speccing the trait-impl
method infeasible (e.g. it would force a spec onto the `sys::mm::Address` trait
declaration affecting all implementors), then instead:

- Produce an **isolated reproducer** (a minimal standalone Verus snippet of just
  the trait-method-spec construct that fails, with the verbatim error), and
- Record this single remaining `assume_specification` in
  `verus-ai-logs/tcb-allowed.md` under "assume_specification for not-yet-verified
  callees", with the precise rationale.

Do not reply with a justification only — either remove the placeholder and show
both `make verify-sys` + `make verify-kernel` clean, or show the isolated
reproducer **and** the tcb-allowed.md entry.

### Secondary concern (will be checked after Item 4)
- **Item 7:** `phys.rs:143-148` (`VERUS DEVIATION`) claims
  "Cross-crate `use_type_invariant` on arch's `FrameNumber` is unsupported
  ('missing type invariant function')". No isolated reproducer file exists for
  this claim. The proof is still *sound* (the needed bound
  `0 <= frame@ <= spec_max()` is taken from `into_raw_value`'s real method
  contract, not from an admit), so this is not blocking — but per checklist item 7
  the limitation claim should be backed by an isolated reproducer or the comment
  softened to "by design (bound supplied via method contract)". Address after
  Item 4.
