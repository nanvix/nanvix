// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
verus! {

impl<T> PageDirectory<T> where T: DerefMut<Target = [PteWord]> + GetPageDirectoryStorage {
    /// Read-only observation of existing tokens; does not mint or transfer their authority.
    pub closed spec fn permissions(&self) -> Map<nat, NanvixPdeToken> {
        self.permissions@
    }
}

} // verus!
