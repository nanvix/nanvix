# Verus-unsupported constructs — `mm::virt::vmem`

## `while let Some(..) = list.pop_front() { .. }`

- **Error:** `The verifier does not yet support the following Rust feature: let expressions`
- **Locations (original):** `Vmem::new` (×2), `impl Drop for Vmem` (×3).
- **Trigger (minimal):**
  ```rust
  while let Some(x) = list.pop_front() { drop(x); }
  ```
- **Resolution:** Verus cannot parse/translate `while let`, which blocks the
  verifier from running on the whole crate. Because the verifier cannot process
  the function at all, an `admit()` placeholder is insufficient (it only skips a
  proof obligation, not a front-end parse failure). The loops were rewritten into
  the exactly-equivalent `loop { let n = list.pop_front(); if n.is_none() { break; }
  let x = n.unwrap(); .. }` form. This preserves runtime semantics bit-for-bit
  (same drain order, same effects), so it does not introduce verified/runtime
  divergence — it only removes a front-end syntax the verifier does not yet accept.
