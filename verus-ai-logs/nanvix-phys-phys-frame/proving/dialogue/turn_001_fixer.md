## Response to Turn 1

### What I Changed

Implemented the reviewer's required fix exactly: made the frame-number representability
bound **target-agnostic** by carrying it in the allocator invariant, and removed the
`global size_of usize == 8` directive that broke 32-bit codegen.

1. **`src/kernel/src/mm/phys/frame.proof.rs`**
   - **Deleted** `global size_of usize == 8;` (and its comment) at the top of the file —
     this was the source of the `error[E0080]: ... does not have the expected size`
     codegen panic on the default `TARGET=x86` build.
   - **Strengthened `Inner::internal_inv`** with the correct representability conjunct:
     ```rust
     // Every managed bitmap index is a representable frame number. Established by `init`,
     // which sizes the bitmap from `MEMORY_SIZE / FRAME_SIZE` (far below
     // `FrameNumber::spec_max()`), so `from_raw_value`/`from_frame_number` never reject an
     // in-range index. Target-agnostic; holds on both 32- and 64-bit builds.
     &&& self.bitmap@.num_bits <= FrameNumber::spec_max() + 1
     ```
     This is the real fact established by the allow-listed TCB boundary (`init`/`instance`,
     which `ensures (*r).inv()`), so it is sound to assume from the trust boundary. It is
     preserved by every proof target because `num_bits` is never resized.

2. **`src/kernel/src/mm/phys/frame.rs`**
   - **`Inner::alloc`** (was lines ~155–181): removed the now-unused exec binding
     `let nbits = self.bitmap.number_of_bits();` and the word-size reasoning
     (`nbits < u32::MAX`, `(u32::MAX as int) <= spec_max() by (compute)`). The conversion
     bound is now discharged directly from the invariant:
     ```rust
     assert(idx < self.bitmap@.num_bits);
     assert(self.bitmap@.num_bits <= FrameNumber::spec_max() + 1);
     assert(idx <= FrameNumber::spec_max());
     ```
   - **`Inner::alloc_contiguous`** (was lines ~340–347): replaced
     `assert((u32::MAX as int) <= FrameNumber::spec_max()) by (compute);` with
     `assert(self.bitmap@.num_bits <= FrameNumber::spec_max() + 1);` so the
     `from_raw_value` `None`-branch is discharged from the invariant rather than the word
     size.

3. **`verus-ai-logs/nanvix-phys-phys-frame/bugs.md`**
   - Marked both the "top-of-memory frame unrepresentable" bug and the
     "`global size_of usize == 8` directive" entries as **RESOLVED**, documenting the
     invariant-based fix (this is a record update reflecting the actual code change, not a
     justification to close the item).

No locked top-level spec (`Inner::share/refcount/is_covered/free/book/alloc_range/
alloc_contiguous/alloc`) was weakened. The change to `internal_inv` is a **strengthening**
(adds a guarantee), not a weakening, and is the change the reviewer explicitly required.
No `external_body` was added.

### Verification

**`make verify-kernel` (default `TARGET=x86`):**
```
verification: 112 verified, 0 errors (exit 0)
```
No `error[E0080]`. The const-eval codegen panic is gone — the command now compiles to
exit 0.

**`make verify-kernel MODULE=mm::phys` (default `TARGET=x86`):**
```
verification: 82 verified, 0 errors (exit 0)
```
(`status: CHEATING_DETECTED` reflects only out-of-scope `admit`s in `manager.proof.rs` /
`mod.proof.rs` and the allow-listed `external_body` wrappers; frame scope has
`admit=0, assume=0`.)

**`./z build` (default `TARGET=x86`, non-`verus_keep_ghost` codegen):**
```
[OK]    Build complete.
EXIT=0
```
(There is no `make build` target in the Makefile; `./z build` is the repository's build
command. It exercises the real non-ghost codegen path, which is where the E0080 would
surface — it does not.)

**`make verify` (all verified crates, default `TARGET=x86`):** all crates exit 0, 0 errors.

**Cross-target `make verify-kernel TARGET=x86_64`:** the frame proofs no longer depend on
word size, but the **whole-kernel x86_64 build does not compile for a pre-existing,
unrelated reason** in an arch module I did not touch:
```
error[E0433]: cannot find `Idt` in `idt`
 (src/kernel/src/hal/arch/x86_64/cpu/idt.rs)
error: could not compile `kernel` (bin "kernel") due to 1 previous error
  0 verified, compilation/setup error (verus did not run)
```
This is an `arch/x86_64` IDT compilation error that aborts the build before Verus runs;
it is independent of `mm::phys::frame` (my diff is confined to `frame.rs` /
`frame.proof.rs` / `bugs.md`). The fix itself is target-agnostic: the representability
bound is now `num_bits <= spec_max() + 1` for whatever `spec_max()` the target defines, so
it no longer assumes `usize == 8`.

### Result: FIXED

The blocking failure (#10) is resolved: `global size_of usize == 8;` is removed, the
representability bound is carried by the (strengthened) `internal_inv`, and on the default
`TARGET=x86`:
- `make verify-kernel` → `112 verified, 0 errors`, **exit 0, no E0080**;
- `./z build` → **exit 0** (Build complete);
- `make verify` → all crates exit 0.

The only remaining cross-target gap (`TARGET=x86_64`) is a pre-existing
`arch/x86_64/cpu/idt.rs` compile error outside this module's scope; it prevents Verus from
running at all and is not caused by, nor fixable within, the frame allocator.
