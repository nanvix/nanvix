# Verification TODOs: phys-upool

None.

All in-scope functions of `src/kernel/src/mm/phys/upool.rs`
(`UserFrame::{new, address, leak, share, refcount}`, `Upool::{new, alloc}`,
plus `<UserFrame as Drop>::drop`) verify with real proofs and no remaining
`admit()` / `assume()` / `external_body` / `cfg`-gated exec code.

Module-scoped check (`make verify-kernel MODULE=mm::phys::upool`) reports:

```
note: verifying module mm::phys::upool
verification results:: 8 verified, 0 errors
✅ No cheating detected in module mm::phys::upool.
status: CLEAN
```
