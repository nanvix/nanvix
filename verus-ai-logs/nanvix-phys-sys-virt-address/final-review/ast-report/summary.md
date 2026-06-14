# AST Consistency Report: ast_orig_25737d_l

**Source:** `/tmp/ast_orig_25737d_l.rs`
**Verus:** `src/libs/sys/src/sys/mm/address/virt.rs`

## Summary

- Functions matched: 17/19
- Functions mismatched: 2
- Missing in Verus: 0
- Extra in Verus: 0
- **Consistent: NO**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `VirtualAddress::align_down` [VirtualAddress__align_down.diff](full/VirtualAddress__align_down.diff) [src](full/VirtualAddress__align_down_source.rs) [verus](full/VirtualAddress__align_down_verus.rs) | MISMATCH | 108-110 | 218-220 |
| `VirtualAddress::from_raw_value` [VirtualAddress__from_raw_value.diff](full/VirtualAddress__from_raw_value.diff) [src](full/VirtualAddress__from_raw_value_source.rs) [verus](full/VirtualAddress__from_raw_value_verus.rs) | MISMATCH | 69-71 | 181-183 |

## Full Diffs (source vs Verus with spec/proof)

Directory: `full/`

| Function | Status | Files |
|----------|--------|-------|
| `VirtualAddress::align_down` | MISMATCH | VirtualAddress__align_down_source.rs, VirtualAddress__align_down_verus.rs, VirtualAddress__align_down.diff |
| `VirtualAddress::from_raw_value` | MISMATCH | VirtualAddress__from_raw_value_source.rs, VirtualAddress__from_raw_value_verus.rs, VirtualAddress__from_raw_value.diff |

## Exec-Only Diffs (source vs Verus stripped of ghost/proof)

Directory: `exec-only/`

These diffs show only the executable code differences, with all Verus
annotations (requires/ensures, proof blocks, ghost variables, invariants)
removed. This makes it easier to spot real exec logic changes.

| Function | Status | Files |
|----------|--------|-------|
| `VirtualAddress::align_down` | MISMATCH | VirtualAddress__align_down_source_stripped.rs, VirtualAddress__align_down_verus_stripped.rs, VirtualAddress__align_down.diff |
| `VirtualAddress::from_raw_value` | MISMATCH | VirtualAddress__from_raw_value_source_stripped.rs, VirtualAddress__from_raw_value_verus_stripped.rs, VirtualAddress__from_raw_value.diff |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `VirtualAddress::add` | MATCH | ✅ |  |
| `VirtualAddress::add_assign` | MATCH | ✅ |  |
| `VirtualAddress::align_down` | MISMATCH | ❌ |  |
| `VirtualAddress::align_up` | MATCH | ✅ |  |
| `VirtualAddress::as_mut_ptr` | MATCH | ✅ |  |
| `VirtualAddress::as_ptr` | MATCH | ✅ |  |
| `VirtualAddress::checked_add` | MATCH | ✅ |  |
| `VirtualAddress::checked_sub` | MATCH | ✅ |  |
| `VirtualAddress::clone_address` | MATCH | ✅ |  |
| `VirtualAddress::fmt` | MATCH | ✅ |  |
| `VirtualAddress::from` | MATCH | ✅ |  |
| `VirtualAddress::from_raw_value` | MISMATCH | ❌ |  |
| `VirtualAddress::into_raw_value` | MATCH | ✅ |  |
| `VirtualAddress::is_aligned` | MATCH | ✅ |  |
| `VirtualAddress::max_addr` | MATCH | ✅ |  |
| `VirtualAddress::new` | MATCH | ✅ |  |
| `u32::from` | MATCH | ✅ |  |
| `u64::from` | MATCH | ✅ |  |
| `usize::from` | MATCH | ✅ |  |

