## Response to Turn 1

### What I Changed

The reviewer flagged the lone outstanding item: the intra-crate
`assume_specification[ <PhysicalAddress as ::sys::mm::Address>::from_raw_value ]` at
`src/kernel/src/hal/mem/types/address/frame.spec.rs:20`, which is required for bottom-up
verification but was **not recorded** in `verus-ai-logs/tcb-allowed.md`.

The reviewer offered two resolutions:
- **Preferred** — supersede it by adding a real `#[verus_spec]` to
  `<PhysicalAddress as ::sys::mm::Address>::from_raw_value` in `phys.rs:185` and deleting
  the block.
- **Fallback** — document the bottom-up placeholder in `tcb-allowed.md`.

I took the **Fallback**, because the Preferred option is **out of my editable scope**.
My hard rule for this task is *"Do not touch unlisted functions"*; the only in-scope
functions are the `FrameAddress` methods. `PhysicalAddress::from_raw_value` lives in the
sibling `hal::mem::types::address::phys` module (a separate verification target), so adding
a `#[verus_spec]` there — and discharging its body proof — would modify unlisted,
out-of-scope code. The reviewer explicitly permits the fallback in this case
("If `phys` is out of your editable scope, use the fallback.").

**File changed:** `verus-ai-logs/tcb-allowed.md` (after line 152, in the
*"`assume_specification` for not-yet-verified callees"* section). Added a new bullet group
**"Intra-crate placeholder in `hal::mem::types::address::frame.spec.rs`"** recording:

```
- `src/kernel/src/hal/mem/types/address/frame.spec.rs::<PhysicalAddress as
  ::sys::mm::Address>::from_raw_value` — `ensures Ok(pa) => pa@ == value as int`,
  `Err(_) => true`. ... its `Address::from_raw_value` body (`phys.rs:185`) currently
  carries no `#[verus_spec]` (verified: `grep -n verus_spec` over `phys.rs:168-187` is
  empty). ... trusted only until the HAL address layer (`phys`) is verified ... removed —
  the same "superseded when the address layer is verified" rationale ...
```

The justification states (as required) that `phys.rs:185` currently has **no**
`#[verus_spec]` — confirmed by `grep -n verus_spec` over `phys.rs:168-187` returning empty.

No source/spec/proof file in the target module was modified; contracts remain byte-identical
(no spec drift).

### Verification

**1. Fix-presence grep (Fallback branch):** new entry present in `tcb-allowed.md`:
```
$ grep -n -A1 "frame.spec.rs::<PhysicalAddress" verus-ai-logs/tcb-allowed.md
156:  - `src/kernel/src/hal/mem/types/address/frame.spec.rs::<PhysicalAddress as
157-    ::sys::mm::Address>::from_raw_value` — `ensures Ok(pa) => pa@ == value as int`,
```

**2. `make verify-kernel MODULE=hal::mem::types::address::frame` → exit 0, module verified:**
```
note: verifying module hal::mem::types::address::frame
verification results:: 6 verified, 0 errors (partial verification with `--verify-*`)
  Exit code : 0
```

**3. `make verify-kernel` (all modules) → exit 0, no cross-module regression:**
```
verify-kernel exit=0
  Exit code : 0
```

**4. `./z build -- all-kernel` → 0 errors, 0 warnings:**
```
build exit=0
    Finished `dev` profile [optimized + debuginfo] target(s) in 0.19s
[OK]    Build complete.
```

`make build` → exit 0 (`Nothing to be done for 'build'`).

The target module's cheating audit is unchanged (admit=0, external_body=0, `assume(...)`=0
in `frame.rs`/`frame.spec.rs`/`frame.proof.rs`); the single `assume_specification` is now
documented in `tcb-allowed.md` exactly as the existing intra-crate placeholders for the
`mm/phys` and `hal::mem` layers are.

### Result: FIXED
