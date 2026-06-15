## Response to Turn 1

### What I Changed

The reviewer's item 7 (FAIL): the two new `assume_specification`s introduced in
`frame.spec.rs` were not registered in `verus-ai-logs/tcb-allowed.md`. This is a
registration/documentation requirement — the contracts themselves are correct and
were left untouched (no weakening, no removal).

**`verus-ai-logs/tcb-allowed.md`** (appended a new section after the
`hal::mem::types::address::phys` library-edge section, lines ~272–308):

- Added section `## Allowed assume_specification — hal::mem::types::address::frame library edge`.
- Registered both declarations by fully-qualified path:
  1. `<crate::hal::mem::types::address::PhysicalAddress as sys::mm::Address>::from_raw_value`
     (declared in `src/kernel/src/hal/mem/types/address/frame.spec.rs:110`).
     Documented its ensures
     (`Ok(r) => r@ == value as int && spec_frame_number(r@) <= spec_max_frame_number(); Err(_) => true`)
     and the justification: the `impl Address for PhysicalAddress`
     (`phys.rs:230`, unspecced) cannot be body-verified in place because sibling
     methods use `usize as *const u8` / `usize as *mut u8` casts the Verus
     front-end rejects (cited `verus-ai-logs/verus-unsupported.md`:
     `error: Verus does not support this cast: \`usize\` to \`*const u8\``);
     per-method `external_body` would pull the whole trait `impl` into scope.
     Noted the value-free `Err` rationale, the `r@`-vs-`spec_addr` cycle
     avoidance, the precedent (`kframe.spec.rs` / `page.spec.rs`), and
     "Removed when `hal::mem` is verified."
  2. `<crate::hal::mem::PageAligned<T> as core::ops::Deref>::deref`
     (declared in the same file, `frame.spec.rs:129`). Documented the
     pure-projection ensures (`spec_addr(result) == addr@`), that it is a
     `core::ops::Deref` trait method below the verification boundary, the
     `spec_addr`-vs-`result@` rationale, precedent `page.spec.rs`, and removal note.

No changes to `frame.rs`, `frame.spec.rs`, or `frame.proof.rs` — the
`assume_specification` contracts are unchanged.

### Verification

`grep -n -i "frame.spec\|PhysicalAddress as\|::deref" verus-ai-logs/tcb-allowed.md`
now shows the two new entries:

```
285:- `<crate::hal::mem::types::address::PhysicalAddress as sys::mm::Address>::from_raw_value`
286:  (declared in `src/kernel/src/hal/mem/types/address/frame.spec.rs`).
301:- `<crate::hal::mem::PageAligned<T> as core::ops::Deref>::deref`
302:  (declared in `src/kernel/src/hal/mem/types/address/frame.spec.rs`).
```

`make verify-kernel MODULE=hal::mem::types::address::frame`:

```
  Exit code : 0
  verification: cached (no recompilation), — (exit 0)
  cheating: assume=0 external_body=24 admit=1 trusted=0 no_decreases=0 cfg_gate=6
  status: CLEAN
```

No new `admit`/`external_body` (still `external_body=24`, all pre-existing;
`admit=1` is the deferred bridge lemma, item 13). `assume=0`.

`./z build` (dual compilation):

```
[OK]    Build complete.
```

### Result: FIXED

Item 7 resolved: both frame-module `assume_specification` boundaries are now
registered in `tcb-allowed.md` with fully-qualified paths, stated ensures, and
per-boundary rationale, mirroring the existing `kframe.spec.rs` / `page.spec.rs`
/ `phys.spec.rs` sections. Verification (exit 0, `assume=0`, CLEAN) and build
(exit 0) both pass with no regression.
