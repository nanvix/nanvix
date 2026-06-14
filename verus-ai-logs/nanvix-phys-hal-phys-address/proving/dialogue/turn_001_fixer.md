## Response to Turn 1

### What I Changed

The reviewer asked to eliminate the `assume_specification[ <VirtualAddress as Address>::into_raw_value ]`
in `phys.spec.rs` by giving the real `sys` trait-impl method a verified spec (as was done for
`VirtualAddress::new`), **or** — if a genuine Verus limitation blocks that — to provide an isolated
reproducer **and** a `tcb-allowed.md` entry.

I tried the real fix first; it hit a genuine, demonstrable Verus limitation, so I took the
explicitly-sanctioned escape-hatch branch. Along the way I found and fixed a **pre-existing
regression** that was already breaking `make verify-sys`.

1. **`src/libs/sys/src/sys/mm/address/virt.rs` (line ~167)** — Removed the `#[verus_verify]`
   attribute from the `impl Address for VirtualAddress` block and replaced it with an explanatory
   note. **This is a bug fix, not a weakening:** git history shows commit `d54fd253d` verified `sys`
   (PASS, 6 verified) with this block **un-annotated**; a later commit **added** `#[verus_verify]`
   to it, which regressed `make verify-sys` to a hard compilation error (HEAD was `c7a556350`,
   "verify FAIL: sys::all"). The annotation forces Verus to verify the whole impl, which contains
   `as_ptr`/`as_mut_ptr` using unsupported `usize as *const u8` / `usize as *mut u8` casts.

2. **`src/kernel/src/hal/mem/types/address/phys.spec.rs` (lines ~57–66)** — Kept the
   `assume_specification` for `into_raw_value` (it is required and cannot be eliminated — see below)
   and expanded its doc-comment to record the precise Verus limitation, the empirical regression
   evidence, and pointers to the reproducers + `tcb-allowed.md`. **No contract changed** (still
   `ensures result as int == addr@`); no spec weakened.

3. **`verus-ai-logs/tcb-allowed.md`** — Added an entry documenting this single retained
   `assume_specification`, with both verbatim error messages and the rationale (verifying it would
   require `external_body` on `as_ptr`/`as_mut_ptr`, *expanding* the TCB to remove one trivial
   assumption — net-negative).

4. **`verus-ai-logs/nanvix-phys-hal-phys-address/specification/whole_impl_rule.rs` and
   `ptr_cast.rs`** — Two minimal standalone Verus reproducers, one per error.

#### Why the placeholder cannot be eliminated (genuine Verus limitation)

`into_raw_value` is a **trait-impl** method. Verus requires the *entire* trait impl to be verified
as a unit (reproducer `whole_impl_rule.rs`):

```
error: In order to verify any items of this trait impl, the entire impl must be verified.
Try wrapping the entire impl in the `verus!` macro.
  --> whole_impl_rule.rs:21:1
```

But the same `impl Address for VirtualAddress` block contains `as_ptr`/`as_mut_ptr` whose
int-to-pointer casts Verus does not support (reproducer `ptr_cast.rs`, and the real
`make verify-sys` failure):

```
error: Verus does not support this cast: `usize` to `*const u8`
  --> ptr_cast.rs:24:37
error: Verus does not support this cast: `usize` to `*mut u8`
```

Unlike its sibling `VirtualAddress::new` (an **inherent** method, freely annotatable), there is no
way to spec `into_raw_value` without verifying the whole block, and the whole block is unverifiable
without `external_body` on the two pointer-cast methods. The `assume_specification`'s body (`self.0`)
trivially satisfies `result as int == self@`, so it is the smallest honest trust boundary.

### Verification

`make verify-sys` (regression fixed — now PASSES):
```
verification results:: 6 verified, 0 errors
  Exit code : 0
  cheating: assume=0 external_body=0 admit=0 trusted=0 no_decreases=0 cfg_gate=0
  status: CLEAN
```

`make verify-kernel MODULE=hal::mem::types::address::phys`:
```
verification results:: 6 verified, 0 errors (partial verification with `--verify-*`)
  Exit code : 0
  ✅ No cheating detected in module hal::mem::types::address::phys.
  Global: assume=0 external_body=11 admit=27 trusted=0 cfg_gate=14
  status: CLEAN
```
(The global `external_body=11`/`admit=27` are pre-existing in out-of-scope `mm/phys/*` modules;
the `phys` address module itself is clean: admit=0, external_body=0, `assume=0`.)

Full `make verify` (all crates): **exit 0**. `sys` = CLEAN. Other crates report
`CHEATING_DETECTED` only from their own pre-existing `admit`/`external_body` (bitmap, nanvix-slab,
bump-allocator, arch, kernel `mm/phys`) — none introduced by this change.

Normal build: `cargo build -p sys` → `Finished` (exit 0). (`make build` is a no-op alias in this
tree; the compiling build checks are the verus crate builds above.)

### Result: FIXED

- The pre-existing `make verify-sys` regression (the breaking `#[verus_verify]` on the trait impl)
  is reverted → `sys` verifies again (6 verified, 0 errors, CLEAN).
- The reviewer's requirement is met via the explicitly-sanctioned escape-hatch branch: the single
  unavoidable `assume_specification` is documented in `tcb-allowed.md`, backed by two isolated
  reproducers reproducing both blocking Verus errors verbatim.
- `phys` module still `6 verified, 0 errors`, `assume=0`, no spec weakened.
