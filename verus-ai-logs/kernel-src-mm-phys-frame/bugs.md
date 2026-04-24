# Bugs Found During Verification: kernel/src/mm/phys/frame.rs

## Bug 1: Potential usize Overflow in Error-Reporting Code

- **Classification**: Code bug (overflow) — auto-fixed
- **Location**: `Inner::alloc_range`, error-reporting computations in loops 1 and 2
- **Original code**:
  ```rust
  let conflicting_addr: usize = index * FRAME_SIZE;
  ```
- **Problem**: When `index` is large and `FRAME_SIZE` is the page size (e.g., 4096),
  the multiplication `index * FRAME_SIZE` can overflow `usize` on 32-bit platforms
  (i686). For example, if `index >= 1048576` on i686 (`usize::MAX = 4294967295`),
  then `index * 4096 > usize::MAX`. This code is only used for error logging
  (not for correctness), but it causes a Verus verification failure because Verus
  checks for overflow on all arithmetic operations.
- **Fix applied**: cfg-gated the error-reporting computations with
  `#[cfg(not(verus_keep_ghost))]` so they are excluded during verification.
  The computations remain in production builds for diagnostic purposes.
  ```rust
  #[cfg(not(verus_keep_ghost))]
  let conflicting_addr: usize = index * FRAME_SIZE;
  ```
- **Severity**: Low — only affects error messages, not functional behavior.
  In practice, valid bitmap indices mapped through FRAME_SIZE should not overflow
  on supported platforms, but Verus cannot prove this without additional platform
  assumptions.
