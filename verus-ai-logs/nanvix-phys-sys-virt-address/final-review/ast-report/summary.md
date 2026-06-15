# AST Consistency Report: ast_orig_t9ybakt0

**Source:** `/home/ruize/nanvix-phy-specs/.review-tmp/ast_orig_t9ybakt0.rs`
**Verus:** `src/libs/sys/src/sys/mm/address/virt.rs`

## Summary

- Functions matched: 18/18
- Functions mismatched: 0
- Missing in Verus: 0
- Extra in Verus: 1
- **Consistent: YES**

## Inconsistent Functions

| Function | Status | Source Lines | Verus Lines |
|----------|--------|-------------|-------------|
| `VirtualAddress::clone_address` [verus](full/VirtualAddress__clone_address_verus.rs) | EXTRA_IN_VERUS |  | 257-259 |

## Full Diffs (source vs Verus with spec/proof)

Directory: `full/`

| Function | Status | Files |
|----------|--------|-------|
| `VirtualAddress::clone_address` | EXTRA_IN_VERUS | VirtualAddress__clone_address_verus.rs (EXTRA) |

## Exec-Only Diffs (source vs Verus stripped of ghost/proof)

Directory: `exec-only/`

These diffs show only the executable code differences, with all Verus
annotations (requires/ensures, proof blocks, ghost variables, invariants)
removed. This makes it easier to spot real exec logic changes.

| Function | Status | Files |
|----------|--------|-------|
| `VirtualAddress::clone_address` | EXTRA_IN_VERUS | VirtualAddress__clone_address_verus.rs (EXTRA) |

## All Functions

| Function | Status | Hash Match | Verification |
|----------|--------|------------|--------------|
| `VirtualAddress::add` | MATCH | ✅ |  |
| `VirtualAddress::add_assign` | MATCH | ✅ |  |
| `VirtualAddress::align_down` | MATCH | ✅ |  |
| `VirtualAddress::align_up` | MATCH | ✅ |  |
| `VirtualAddress::as_mut_ptr` | MATCH | ✅ |  |
| `VirtualAddress::as_ptr` | MATCH | ✅ |  |
| `VirtualAddress::checked_add` | MATCH | ✅ |  |
| `VirtualAddress::checked_sub` | MATCH | ✅ |  |
| `VirtualAddress::fmt` | MATCH | ✅ |  |
| `VirtualAddress::from` | MATCH | ✅ |  |
| `VirtualAddress::from_raw_value` | MATCH | ✅ |  |
| `VirtualAddress::into_raw_value` | MATCH | ✅ |  |
| `VirtualAddress::is_aligned` | MATCH | ✅ |  |
| `VirtualAddress::max_addr` | MATCH | ✅ |  |
| `VirtualAddress::new` | MATCH | ✅ |  |
| `u32::from` | MATCH | ✅ |  |
| `u64::from` | MATCH | ✅ |  |
| `usize::from` | MATCH | ✅ |  |
| `VirtualAddress::clone_address` | EXTRA_IN_VERUS | ❌ |  |

