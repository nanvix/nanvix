## Response to Turn 1

### What I Changed
- `src/libs/sys/src/sys/mm/address/virt.rs` — **attempted** the reviewer's exact
  change (annotate `impl Address for VirtualAddress` with `#[verus_verify]` and add
  `#[verus_spec(... ensures result as int == self@)]` to `into_raw_value`). This
  does **not** compile under Verus (errors below), so it was reverted to the last
  PASS state (trait impl unannotated; `into_raw_value` covered by the
  trait-declaration contract in `mod.rs:51-55`).
- `verus-ai-logs/nanvix-phys-sys-virt-address/verus-unsupported.md` — **new**:
  records the two genuine Verus front-end limitations that block the requested
  change (full exact errors).

No exec code was rewritten and no `external_body`/`assume_specification` was added
to the module's own functions.

### Why the requested change cannot be applied

Verus verifies a trait impl **all-or-nothing**:
`error: In order to verify any items of this trait impl, the entire impl must be
verified.` So annotating `impl Address for VirtualAddress` to body-check
`into_raw_value` forces Verus to also translate every sibling method. Two of them
are unverifiable under the current constraints:

**Attempt 1 — reviewer's exact change** (`#[verus_verify]` on the impl +
`#[verus_spec]` on `into_raw_value`):
```
error: Verus does not support this cast: `usize` to `*const u8`
   --> src/libs/sys/src/sys/mm/address/virt.rs:268:9
    |
268 |         self.0 as *const u8
    |         ^^^^^^^^^^^^^^^^^^^
error: Verus does not support this cast: `usize` to `*mut u8`
   --> src/libs/sys/src/sys/mm/address/virt.rs:272:9
    |
272 |         self.0 as *mut u8
    |         ^^^^^^^^^^^^^^^^^
error: could not compile `sys` (lib) due to 2 previous errors
```
`as_ptr`/`as_mut_ptr` (out of scope) use a `usize as *const/*mut u8` cast that the
Verus front-end (`0.2026.05.24`) does not support. The only Verus-supported
alternative, `vstd::raw_ptr::with_exposed_provenance`, needs an extra
`Tracked<IsExposed>` argument — an exec **signature/body rewrite** of an unlisted
function (forbidden by `verus-constraints` and the hard rules).

**Attempt 2 — temporarily neutralized the casts** to expose the next blocker:
```
error: cannot use function `...::VirtualAddress::align_down` which is ignored
because it is either declared outside the verus! macro or it is marked as `external`.
   --> src/libs/sys/src/sys/mm/address/virt.rs:221:12
error: cannot use function `...::VirtualAddress::is_aligned` ... (virt.rs:239:12)
error: cannot use function `...::VirtualAddress::align_up`   ... (virt.rs:202:9)
error: could not compile `sys` (lib) due to 3 previous errors
```
The trait `align_up`/`align_down`/`is_aligned` delegate to the inherent
`VirtualAddress` align methods, which are out of scope. Resolving this needs specs
on the unlisted `align_up`/`align_down`/`is_aligned` (and transitively
`sys::mm::alignment::*`) — forbidden by the task ("Do not touch unlisted
functions") and by the reviewer ("Do NOT add a spec to the alignment/`checked_*`
methods in this phase").

These two requirements are mutually exclusive with the task's hard rules; there is
no compliant way to body-verify `into_raw_value` inside the trait impl.

### The caller-required property is already proven (without the impl annotation)
The `into_raw_value` contract the reviewer asks for already exists on the **trait
declaration** — `src/libs/sys/src/sys/mm/address/mod.rs:51-55`:
```rust
#[verus_spec(result =>
    ensures
        result as int == self@,
)]
fn into_raw_value(self) -> usize;
```
`Address` is `#[verus_verify]`, so every implementer (including `VirtualAddress`)
is bound by `result as int == self@`. Combined with the inherent `from_raw_value`
ensures (`result@ == raw_addr as int`), the caller round-trip
`from_raw_value(x).into_raw_value() == x` is provable, and callers
(`mm/mmio.rs`, `pm/sync.rs`) consume exactly this. The reviewer's grep checked only
`virt.rs`; the ensures lives at the trait declaration. (The kernel crate already
relies on the same contract via the `assume_specification` in `phys.spec.rs:58`.)

### Verification
`make verify-sys MODULE=sys::sys::mm::address::virt` (restored PASS state):
```
=== Summary ===
  verification: 2 verified, 0 errors (exit 0)
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=1
  coverage: 2/23 exec functions have contracts
  status: CLEAN
```
`make verify` (full, no regressions): `35 verified` (sys) + `60 verified` (kernel),
`0 errors`, exit 0. Normal `cargo build` of `sys` compiles (the pre-existing
duplicate `vstd` import warning at `virt.rs:8`/`:25` is unrelated and present in the
last PASS commit).

### Result: NEEDS_GUIDANCE
The reviewer's exact change does not compile under Verus (Attempt 1 errors above),
and pushing it through (Attempt 2) requires touching unlisted functions and adding a
TCB-disallowed `external_body` — both forbidden by the task's hard rules and by the
reviewer's own "keep the change limited to `into_raw_value`" instruction.

Please choose one:
1. **Accept the trait-declaration contract** (`mod.rs:51-55`) as the verified
   `into_raw_value` guarantee — the round-trip property is proven and consumed by
   callers; no impl-body check is achievable for a trait method whose siblings use
   unsupported ptr casts. (Recommended — keeps `make verify` green.)
2. **Grant explicit exceptions** to allow body-verifying the whole
   `impl Address for VirtualAddress`: (a) add `as_ptr`/`as_mut_ptr` to
   `tcb-allowed.md` so they may be `external_body` (the `usize`→ptr cast is a real
   Verus limitation), **and** (b) authorize adding `#[verus_spec]` to the unlisted
   inherent `align_up`/`align_down`/`is_aligned` plus `sys::mm::alignment::*`. With
   both exceptions I can complete the impl-level annotation.
