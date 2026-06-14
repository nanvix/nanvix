# Verus-Unsupported Constructs — `arch::x86::mem::paging` (`mod.rs`)

This file records language/library constructs in the `paging` module (`mod.rs`)
that Verus cannot translate or verify, together with the minimal reproduction and
the trust-boundary mitigation used.

## 1. Inline assembly (`core::arch::asm!`)

### Where
- `invlpg` — the entire body is a single `core::arch::asm!` block issuing the
  `invlpg ({0})` instruction, which flushes the CPU TLB entry for `vaddr`:
  ```rust
  pub unsafe fn invlpg(vaddr: usize) {
      core::arch::asm!(
          "invlpg ({0})",
          in(reg) vaddr,
          options(nostack, preserves_flags, att_syntax)
      );
  }
  ```

### Minimal reproduction
```rust
use vstd::prelude::*;
#[verus_verify]
pub unsafe fn flush(vaddr: usize) {
    core::arch::asm!("invlpg ({0})", in(reg) vaddr, options(nostack, att_syntax));
}
```

### Exact error
```
error: The verifier does not yet support the following Rust feature: inline-asm expressions
  --> src/libs/arch/src/x86/mem/paging/mod.rs:72:5
   |
72 | /     core::arch::asm!(
73 | |         "invlpg ({0})",
74 | |         in(reg) vaddr,
75 | |         options(nostack, preserves_flags, att_syntax)
76 | |     );
   | |_____^
```

### Why it is unsupported
Verus has no model for inline assembly. Inline asm is explicitly an
external-bottom trust boundary (hardware instructions / registers / MMIO): its
effect — here, invalidating a cached virtual→physical translation in the CPU's
Translation Lookaside Buffer — is hardware microarchitectural state that lives
entirely outside Verus' memory model. There is no `PointsTo`-style permission
token or value a caller can read back, so the body cannot be verified.

### Mitigation (trust boundary)
`invlpg` is marked `#[verus_verify(external_body)]` and recorded in
`verus-ai-logs/tcb-allowed.md`. This is the same external-bottom hardware
boundary already used by `table::read` / `table::write` (volatile page-table
access) and `frame::instance` / `bump_allocator::alloc` (int-to-pointer
materialization). Only the *body* (the inline-asm TLB flush) is trusted.

### Contract
The faithful contract is **empty**: no `requires` (any `usize` is accepted; the
instruction is defined for every operand and is a no-op when no matching TLB
entry exists), and a trivial `ensures` (the function returns `()` with no
Rust-visible effect). Because `invlpg` touches no page tables, frames, or any
Rust-visible state, it provably preserves every caller-side invariant
(page-table well-formedness, mapping counts, allocator state). This exactly
matches the inherited upstream contract
`src/kernel/src/mm/virt/identity_map.spec.rs:151`:
`pub assume_specification[ ::arch::mem::paging::invlpg ](vaddr: usize);`
(no `requires`/`ensures`). No exec signature changed.

### Deferred work
None. The TLB is unobservable in Verus' memory model (see
`view_design.md`); there is no abstract state to carry across the call and no
stronger postcondition to prove later. The empty contract is the final,
faithful specification — not a placeholder.
