use vstd::prelude::*;

verus! {
proof fn test_get_mut_spec(m: &mut std::collections::BTreeMap<u64, u64>) {
    let _r = m.get_mut(&0u64);
}
}

fn main() {}
