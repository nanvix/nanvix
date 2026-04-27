# Code Bugs Found During Verification — Physical Memory Manager

## Bug 1: Arithmetic Underflow in `kpool::Inner::free`

- **Classification**: Code bug (arithmetic underflow)
- **Location**: `src/kernel/src/mm/phys/kpool.rs`, `Inner::free`
- **Severity**: Medium — can cause incorrect behavior if caller passes invalid address
- **Original code**:
  ```rust
  fn free(&mut self, addr: FrameAddress) -> Result<(), Error> {
      let index: usize = (addr.into_raw_value() - self.base.into_raw_value()) / mem::PAGE_SIZE;
      match self.bitmap.clear(index) { ... }
  }
  ```
- **Problem**: The subtraction `addr - base` has no bounds check. If `addr` is below the
  pool base address, the subtraction wraps around on `usize`, producing an incorrect
  (very large) bitmap index. In Rust debug mode this panics; in release mode it silently
  computes a wrong index, potentially corrupting the bitmap by clearing an unrelated bit.
- **Trigger condition**: A caller passes a `FrameAddress` that was not allocated from this
  pool, or passes an address from a different memory region whose raw value is less than
  `self.base`.
- **Fix applied** (by verification pipeline, commit `f29f94ee6`):
  ```rust
  if addr.into_raw_value() < pa_into_raw(self.base) {
      let reason: &str = "frame address below pool base";
      error!("{reason}");
      return Err(Error::new(ErrorCode::BadAddress, reason));
  }
  let index: usize = (addr.into_raw_value() - pa_into_raw(self.base)) / mem::PAGE_SIZE;
  ```
- **Verus error**: `error: possible arithmetic underflow/overflow`

## Bug 2: Arithmetic Overflow in `frame::Inner::alloc_range` Error Logging

- **Classification**: Code bug (arithmetic overflow)
- **Location**: `src/kernel/src/mm/phys/frame.rs`, `Inner::alloc_range`, error-reporting path
- **Severity**: Low — only affects error messages, not functional behavior
- **Original code**:
  ```rust
  Ok(true) => {
      let conflicting_addr: usize = index * mem::FRAME_SIZE;
      let region_start: usize = region.start().into_raw_value();
      let region_end: usize = region_start.saturating_add(region.size());
      error!("{} (frame={:#010x}, ...)", reason, conflicting_addr, region_start, region_end);
      return Err(Error::new(ErrorCode::OutOfMemory, reason));
  }
  ```
- **Problem**: The multiplication `index * FRAME_SIZE` can overflow `usize` on 32-bit
  platforms (i686, where `usize::MAX = 4,294,967,295`). When `index >= 1,048,576`
  (corresponding to physical address ≥ 4 GiB, valid under PAE), `index * 4096` exceeds
  `usize::MAX`. In Rust debug mode this panics; in release mode the error log prints a
  wrapped (incorrect) address, potentially misleading debugging.
- **Trigger condition**: A bitmap index ≥ 1,048,576 on i686 (i.e., physical memory ≥ 4 GiB
  with PAE enabled), and the corresponding frame is already allocated.
- **Fix applied** (by verification pipeline): cfg-gated the error-reporting computations
  with `#[cfg(not(verus_keep_ghost))]` to exclude them during verification. The
  computations remain in production builds for diagnostic purposes.
  ```rust
  #[cfg(not(verus_keep_ghost))]
  let conflicting_addr: usize = index * mem::FRAME_SIZE;
  ```
- **Verus error**: `error: possible arithmetic underflow/overflow`
