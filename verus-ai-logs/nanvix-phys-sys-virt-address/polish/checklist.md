# Polish Report: sys-virt-address

Target file: `src/libs/sys/src/sys/mm/address/virt.rs`
Spec/proof files: `virt.spec.rs`, `virt.proof.rs` (both empty: `verus! { }`)
In-scope functions: `VirtualAddress::into_raw_value`, `VirtualAddress::from_raw_value`,
`VirtualAddress::new`, `VirtualAddress` (struct + `View`).

## Proof Extraction
- Blocks extracted: 0
  - `check_proof_blocks.py virt.rs --all` reports `No proof blocks found.` and
    `No loop invariants found.` There are no inline `proof { ... }` blocks, no
    `assert ... by(...)` hints, and no loop invariants anywhere in the source,
    spec, or proof files.
- Blocks kept inline: 0
  - Nothing inline to keep; the in-scope functions (`new`, inherent
    `from_raw_value`) are proved directly by their `#[verus_spec]` ensures with
    no proof scaffolding required.

## Minimization
- Redundant assertions removed: 0
  - The only `assert` token in the file is the compile-time
    `static_assert::assert_eq_size!` macro (line 40), which is not a Verus proof
    assertion and is unrelated to verification.
- Redundant lemmas/hints removed: 0
  - `virt.proof.rs` contains no lemmas; `virt.spec.rs` contains no spec
    functions. No `by(...)`, `by(nonlinear_arith)`, trigger, or `reveal`
    annotations exist to prune.
- Dead spec functions removed: 0
  - The single `closed spec fn view` (in the `View for VirtualAddress` impl) is
    live: it backs `result@` in the `ensures` of both `new`
    (`result@ == value as int`) and inherent `from_raw_value`
    (`result@ == raw_addr as int`). Not removable.
- Debug artifacts removed: 0
  - No TODO/FIXME, commented-out code, or property-ID tags
    (e.g. `// FUNC-POST-n`, `// INV-n`) present. The `// Material for
    verification` banner (line 316) is an original structural section header and
    is retained per the proof-minimization rule to preserve original source
    comments.

## Result
The verification for `virt.rs` was already in a maximally lean, integration-ready
state; no extraction or minimization edits were necessary.

- `make verify-sys` → `status: CLEAN`, exit 0
  (`2 verified, 0 errors`; `assume=0 external_body=0 admit=0`).
- `cargo build --locked -p sys` → `Finished` (build passes).
- Source/spec/proof files left byte-for-byte unchanged.
