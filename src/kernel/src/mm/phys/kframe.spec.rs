// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

verus! {

impl KernelFrame {
    pub closed spec fn base_address(&self) -> int {
        self.base@
    }
}

/// Creates raw page-table entry permissions for a newly allocated kernel frame.
#[verifier::external_body]
pub(super) proof fn mint_kernel_frame_page_table_permissions(
    base: int,
) -> (tracked permissions: Map<nat, PointsTo<PteWord>>)
    ensures
        permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH,
        forall|i: nat| permissions.dom().contains(i)
            <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH,
        forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
            let permission = #[trigger] permissions[i];
            &&& permission.ptr()@.addr as int == base + i * 4
            &&& permission.is_uninit()
        },
{
    unimplemented!()
}

} // verus!
