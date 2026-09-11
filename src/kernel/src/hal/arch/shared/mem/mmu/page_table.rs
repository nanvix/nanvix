// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::{
        AccessPermission,
        Address,
        FrameAddress,
        PageAddress,
        PageAligned,
        PhysicalAddress,
    },
    mm::GetPageTableStorage,
};
use ::arch::mem::paging::{
    AccessedFlag,
    CopyOnWriteFlag,
    DirtyFlag,
    FrameNumber,
    PageCacheDisableFlag,
    PageTableEntry,
    PageTableEntryFlags,
    PageWriteThroughFlag,
    PresentFlag,
    PteWord,
    ReadWriteFlag,
    UserSupervisorFlag,
};
use ::core::ops::DerefMut;
use ::sys::error::{
    Error,
    ErrorCode,
};

include!("page_table.spec.rs");

//==================================================================================================
// Structures
//==================================================================================================
///
/// # Description
///
/// A type that represents a page table.
///
#[verus_verify]
pub struct PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    /// Number of pages mapped in the page table.
    nmapped: usize,
    /// Entries.
    entries: T,
    /// Specification tokens for page-table entries.
    #[cfg(verus_keep_ghost_body)]
    permissions: Tracked<Map<nat, NanvixPteToken>>,
}

verus! {

impl<T> PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    pub closed spec fn permissions(&self) -> Map<nat, NanvixPteToken>
    {
        self.permissions@
    }
}

} // end verus!

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl<T> PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        with
            Tracked(raw_permissions):
                Tracked<Map<nat, PointsTo<PteWord>>>,
        requires
            0 <= entries.get_storage().physical_base_address(),
            entries.get_storage().physical_base_address()
                % (::arch::mem::PAGE_SIZE as int) == 0,
            raw_permissions.dom().len() == ::arch::mem::PAGE_TABLE_LENGTH,
            forall|i: nat| raw_permissions.dom().contains(i)
                <==> 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH,
            forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                let permission = #[trigger] raw_permissions[i];

                permission.ptr()@.addr as int
                    == entries.get_storage().entries_base_address()
                        + i * 4
                    && permission.is_uninit()
            },
        ensures
            result.ready_for_mmu(),
            forall|i: nat| 0 <= i < ::arch::mem::PAGE_TABLE_LENGTH ==> {
                &&& result.permissions().contains_key(i)
                &&& (#[trigger] result.permissions()[i]).expected() == Some(0)
            },
    )]
    pub fn new(entries: T) -> Self {
        let mut page_table: Self = Self {
            nmapped: 0,
            entries,
            #[cfg(verus_keep_ghost_body)]
            permissions: Tracked::new(mint_nanvix_pte_tokens(raw_permissions)),
        };
        page_table.clean();
        page_table
    }
}

impl<T> PageTable<T>
where
    T: DerefMut<Target = [PteWord]> + GetPageTableStorage,
{
    ///
    /// # Description
    ///
    /// Creates a page table wrapper around existing (already-populated) storage.
    ///
    /// Unlike [`new`](Self::new), this does not zero the entries.
    ///
    /// # Returns
    ///
    /// A new [`PageTable`] instance that wraps the provided storage.
    ///
    /// Returns the number of pages mapped in the page table.
    pub fn nmapped(&self) -> usize {
        self.nmapped
    }

    /// Maps a physical address into a virtual address in the target page table.
    pub fn map(
        &mut self,
        vaddr: PageAddress,
        paddr: FrameAddress,
        supervisor: bool,
        writethrough: bool,
        cache: bool,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(vaddr) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!(
                    "map(): {} (vaddr={:?}, paddr={:?}, supervisor={:?}, writethrough={:?}, \
                     cache={:?}, access={:?})",
                    reason, vaddr, paddr, supervisor, writethrough, cache, access
                );
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page table entry is busy.
        if pte.is_present() {
            let reason: &str = "page table entry is busy";
            error!(
                "map(): {} (vaddr={:?}, paddr={:?}, supervisor={:?}, writethrough={:?}, \
                 cache={:?}, access={:?})",
                reason, vaddr, paddr, supervisor, writethrough, cache, access
            );
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Construct page table entry.
        let pte: PageTableEntry = PageTableEntry::new(
            PageTableEntryFlags::new(
                PresentFlag::Present,
                if access.is_writable() {
                    ReadWriteFlag::ReadWrite
                } else {
                    ReadWriteFlag::ReadOnly
                },
                if supervisor {
                    UserSupervisorFlag::Supervisor
                } else {
                    UserSupervisorFlag::User
                },
                if writethrough {
                    PageWriteThroughFlag::WriteThrough
                } else {
                    PageWriteThroughFlag::NotWriteThrough
                },
                if cache {
                    PageCacheDisableFlag::CacheEnabled
                } else {
                    PageCacheDisableFlag::CacheDisabled
                },
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            ),
            paddr.into_frame_number(),
        );

        // Write page table entry.
        self.write_pte(vaddr, pte);

        self.nmapped += 1;

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Unmaps a page address from the target page table.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address to unmap.
    ///
    /// # Return Values
    ///
    ///Upon success, the frame address that was associated with the given page address is returned
    ///and the concerned page is unmapped from the target page table. Upon failure, an error is
    ///returned instead.
    ///
    pub fn unmap(&mut self, page_address: PageAddress) -> Result<FrameAddress, Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("{reason} (page_address={page_address:?})");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("{reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::ResourceBusy, reason));
        }

        // Retrieve frame address.
        let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;

        // Construct page table entry.
        let pte: PageTableEntry = PageTableEntry::new(
            PageTableEntryFlags::new(
                PresentFlag::NotPresent,
                ReadWriteFlag::ReadOnly,
                UserSupervisorFlag::User,
                PageWriteThroughFlag::NotWriteThrough,
                PageCacheDisableFlag::CacheDisabled,
                AccessedFlag::NotAccessed,
                DirtyFlag::NotDirty,
            ),
            FrameNumber::NULL,
        );

        // Write page table entry.
        self.write_pte(page_address, pte);

        // Invalidate the TLB entry so the CPU does not use a stale mapping to the
        // old frame if this virtual address is re-mapped to a different frame.
        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(page_address.into_raw_value()) };

        self.nmapped -= 1;

        Ok(paddr)
    }

    ///
    /// # Description
    ///
    /// Looks up a page address in the target page table.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address to lookup.
    ///
    /// # Return Values
    ///
    /// Upon success, the frame address associated with the target page is returned. Upon failure,
    /// an error is returned instead.
    ///
    pub fn lookup(&self, page_address: PageAddress) -> Result<FrameAddress, Error> {
        // Obtain a cached copy of the page table entry.
        let pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("{reason} (page_address={page_address:?})");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("{reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Retrieve frame address.
        let paddr: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;

        Ok(paddr)
    }

    ///
    /// # Description
    ///
    /// Checks whether a page is present in the target page table.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address to check.
    ///
    /// # Returns
    ///
    /// - `Ok(true)` if the page is present.
    /// - `Ok(false)` if the page is not present.
    /// - `Err(_)` if the page table entry could not be read.
    ///
    pub fn is_page_present(&self, page_address: PageAddress) -> Result<bool, Error> {
        match self.read_pte(page_address) {
            Some(pte) => Ok(pte.is_present()),
            None => {
                let reason: &str = "failed to read page table entry";
                error!("{reason} (page_address={page_address:?})");
                Err(Error::new(ErrorCode::TryAgain, reason))
            },
        }
    }

    /// Changes access permissions on a page.
    pub fn ctrl(
        &mut self,
        supervisor: bool,
        page_address: PageAddress,
        access: AccessPermission,
    ) -> Result<(), Error> {
        // Obtain a cached copy of the page table entry.
        let mut pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!(
                    "change_access_permissions(): {} (page_address={:?}, supervisor={:?}, \
                     access={:?})",
                    reason, page_address, supervisor, access
                );
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        // Check if page is not present.
        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!(
                "change_access_permissions(): {} (page_address={:?}, supervisor={:?}, access={:?})",
                reason, page_address, supervisor, access
            );
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        // Modify page table entry.
        if access.is_writable() {
            pte.set_read_write(ReadWriteFlag::ReadWrite);
        } else {
            pte.set_read_write(ReadWriteFlag::ReadOnly);
        }
        if supervisor {
            pte.set_user_supervisor(UserSupervisorFlag::Supervisor);
        } else {
            pte.set_user_supervisor(UserSupervisorFlag::User);
        }

        // Write page table entry.
        self.write_pte(page_address, pte);

        // Invalidate the TLB entry so the permission change takes effect immediately.
        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(page_address.into_raw_value()) };

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Marks a present user page as copy-on-write: clears the writable bit and sets the AVL
    /// copy-on-write bit.
    ///
    /// After this call, any user-mode write to the page faults with `P=1, W=1, U=1`,
    /// which the in-kernel page-fault handler recognizes (via the AVL bit) as a
    /// copy-on-write fault.
    ///
    /// The page must be currently writable. Pages that are already read-only must not
    /// be marked copy-on-write: stamping the AVL copy-on-write bit on a genuinely
    /// read-only mapping would cause a subsequent user-mode write to be silently
    /// resolved as a copy-on-write fault instead of raising a protection fault.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address of the entry to mark.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn mark_cow(&mut self, page_address: PageAddress) -> Result<(), Error> {
        let mut pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("mark_cow(): {reason} (page_address={page_address:?})");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("mark_cow(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        if !pte.flags().is_writable() {
            let reason: &str = "page is already read-only";
            error!("mark_cow(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        pte.set_read_write(ReadWriteFlag::ReadOnly);
        pte.set_cow(CopyOnWriteFlag::CopyOnWrite);
        self.write_pte(page_address, pte);

        // Invalidate the TLB entry so the new copy-on-write permissions take effect immediately.
        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(page_address.into_raw_value()) };

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Clears the copy-on-write mark on a present user page: clears the AVL copy-on-write
    /// bit and restores the writable bit. Inverse of [`Self::mark_cow`].
    ///
    /// The PTE must currently be present and marked copy-on-write.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address of the entry to unmark.
    ///
    /// # Returns
    ///
    /// Upon success, `Ok(())` is returned. Upon failure, an error is returned instead.
    ///
    pub fn unmark_cow(&mut self, page_address: PageAddress) -> Result<(), Error> {
        let mut pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("unmark_cow(): {reason} (page_address={page_address:?})");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("unmark_cow(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        if !pte.is_cow() {
            let reason: &str = "page is not copy-on-write";
            error!("unmark_cow(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        pte.set_cow(CopyOnWriteFlag::NotCopyOnWrite);
        pte.set_read_write(ReadWriteFlag::ReadWrite);
        self.write_pte(page_address, pte);

        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(page_address.into_raw_value()) };

        Ok(())
    }

    ///
    /// # Description
    ///
    /// Resolves a copy-on-write mapping by repointing the PTE at `new_frame`,
    /// clearing the AVL copy-on-write bit, and restoring the writable bit.
    ///
    /// The PTE must currently be present and marked copy-on-write.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address of the entry to resolve.
    /// - `new_frame`: New physical frame to install in the PTE.
    ///
    /// # Returns
    ///
    /// Upon success, the previous frame address (the one the PTE pointed at before
    /// the swap) is returned, so the caller can release its reference. Upon failure,
    /// an error is returned instead.
    ///
    pub fn replace_cow_frame(
        &mut self,
        page_address: PageAddress,
        new_frame: FrameAddress,
    ) -> Result<FrameAddress, Error> {
        let pte: PageTableEntry = match self.read_pte(page_address) {
            Some(pte) => pte,
            None => {
                let reason: &str = "failed to read page table entry";
                error!("replace_cow_frame(): {reason} (page_address={page_address:?})");
                return Err(Error::new(ErrorCode::TryAgain, reason));
            },
        };

        if !pte.is_present() {
            let reason: &str = "page is not present";
            error!("replace_cow_frame(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::NoSuchEntry, reason));
        }

        if !pte.is_cow() {
            let reason: &str = "page is not copy-on-write";
            error!("replace_cow_frame(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        if pte.flags().is_writable() {
            let reason: &str = "copy-on-write page is unexpectedly writable";
            error!("replace_cow_frame(): {reason} (page_address={page_address:?})");
            return Err(Error::new(ErrorCode::InvalidArgument, reason));
        }

        let old_frame: FrameAddress = FrameAddress::from_frame_number(pte.frame_number())?;

        let mut new_flags: PageTableEntryFlags = pte.flags();
        new_flags.set_read_write(ReadWriteFlag::ReadWrite);
        new_flags.set_cow(CopyOnWriteFlag::NotCopyOnWrite);
        let new_pte: PageTableEntry = PageTableEntry::new(new_flags, new_frame.into_frame_number());
        self.write_pte(page_address, new_pte);

        // SAFETY: called from kernel mode after modifying a PTE.
        unsafe { ::arch::mem::paging::invlpg(page_address.into_raw_value()) };

        Ok(old_frame)
    }

    ///
    /// # Description
    ///
    /// Bulk-fills page table entries for contiguous identity-mapped physical memory.
    ///
    /// Each entry maps physical frame `base_frame + i` with the given PTE flags.
    ///
    /// # Parameters
    ///
    /// - `start_index`: First entry index to fill (0–1023).
    /// - `count`: Number of consecutive entries to fill.
    /// - `base_address`: Page-aligned physical address of the first frame.
    /// - `pte_flags`: Strongly typed PTE flags.
    /// - `skip_pte_verification`: If `true`, skip the check that all target entries are not
    ///   present.
    ///
    /// # Returns
    ///
    /// Upon success, the number of frames mapped is returned and the page table entries are
    /// filled. Upon failure, a tuple containing the number of frames that were successfully
    /// mapped before the error and the error itself is returned.
    ///
    /// # Errors
    ///
    /// - `InvalidArgument` if `start_index + count` overflows or exceeds the table length.
    /// - `InvalidArgument` if the present bit is not set in `pte_flags`.
    /// - `InvalidArgument` if a frame number exceeds the valid range.
    /// - `ResourceBusy` if any target entry is already present (unless `skip_pte_verification` is
    ///   `true`).
    ///
    /// # Notes
    ///
    /// - Entries written before a mid-fill error are not rolled back.
    ///
    pub fn fill(
        &mut self,
        start_index: usize,
        count: usize,
        base_address: FrameAddress,
        pte_flags: PageTableEntryFlags,
        skip_pte_verification: bool,
    ) -> Result<usize, (usize, Error)> {
        // Bounds check.
        let end: usize = start_index.checked_add(count).ok_or_else(|| {
            let reason: &str = "index overflow";
            error!("fill(): {}", reason);
            (0, Error::new(ErrorCode::InvalidArgument, reason))
        })?;
        if end > self.entries.len() {
            let reason: &str = "index out of bounds";
            error!(
                "fill(): {} (start_index={}, count={}, entries_len={})",
                reason,
                start_index,
                count,
                self.entries.len()
            );
            return Err((0, Error::new(ErrorCode::InvalidArgument, reason)));
        }

        // Validate that the present bit is set.
        if !pte_flags.is_present() {
            let reason: &str = "present bit not set in pte_flags";
            error!("fill(): {}", reason);
            return Err((0, Error::new(ErrorCode::InvalidArgument, reason)));
        }

        // Verify that all target entries are not present.
        if !skip_pte_verification {
            for index in start_index..end {
                // if PresentFlag::is_set(self.entries[index]) {
                if PresentFlag::is_set(self.env_interaction_read_page_table_entry(index)) {
                    let reason: &str = "page table entry is busy";
                    error!("fill(): {}", reason);
                    return Err((0, Error::new(ErrorCode::ResourceBusy, reason)));
                }
            }
        }

        // Build and write each page table entry.
        let base_frame: FrameNumber = base_address.into_frame_number();
        for i in 0..count {
            let raw_frame: usize = base_frame.into_raw_value().checked_add(i).ok_or_else(|| {
                let reason: &str = "frame number overflow";
                error!("fill(): {}", reason);
                (i, Error::new(ErrorCode::InvalidArgument, reason))
            })?;
            let frame: FrameNumber = FrameNumber::from_raw_value(raw_frame).ok_or_else(|| {
                let reason: &str = "frame number out of range";
                error!("fill(): {}", reason);
                (i, Error::new(ErrorCode::InvalidArgument, reason))
            })?;
            let pte: PageTableEntry = PageTableEntry::new(pte_flags, frame);
            // self.entries[start_index + i] = pte.into_raw_value();
            self.env_interaction_write_page_table_entry(start_index + i, pte.into_raw_value());
            self.nmapped += 1;
        }

        Ok(count)
    }

    fn clean(&mut self) {
        // for pte in self.entries.iter_mut() {
        //     *pte = 0;
        // }
        self.env_interaction_clear_page_table();
    }

    fn read_pte(&self, vaddr: PageAddress) -> Option<PageTableEntry> {
        let pte_idx: usize = vaddr.get_pte_index();
        // let pte: Option<PageTableEntry> = PageTableEntry::from_raw_value(self.entries[pte_idx]);
        let pte: Option<PageTableEntry> =
            PageTableEntry::from_raw_value(self.env_interaction_read_page_table_entry(pte_idx));
        pte
    }

    fn write_pte(&mut self, vaddr: PageAddress, pte: PageTableEntry) {
        let pte_idx: usize = vaddr.get_pte_index();
        // self.entries[pte_idx] = pte.into_raw_value();
        self.env_interaction_write_page_table_entry(pte_idx, pte.into_raw_value());
    }

    ///
    /// # Description
    ///
    /// Reads the page table entry at the given page address, if it is present.
    ///
    /// # Parameters
    ///
    /// - `page_address`: Page address of the entry to read.
    ///
    /// # Returns
    ///
    /// - `Some(pte)` if the entry is present.
    /// - `None` if the entry is not present (or could not be decoded).
    ///
    pub fn read_pte_at(&self, page_address: PageAddress) -> Option<PageTableEntry> {
        match self.read_pte(page_address) {
            Some(pte) if pte.is_present() => Some(pte),
            _ => None,
        }
    }

    pub fn physical_address(&self) -> Result<FrameAddress, Error> {
        let vaddr: usize = self.entries.as_ptr() as usize;
        let paddr: usize = crate::hal::platform::virt_to_phys(vaddr);
        Ok(FrameAddress::new(PageAligned::from_address(PhysicalAddress::from_raw_value(paddr)?)?))
    }

    ///
    /// # Description
    ///
    /// Iterates over the present entries in the target page table.
    ///
    /// # Returns
    ///
    /// An iterator that yields, for each present entry, a tuple `(pte_index, pte)` where
    /// `pte_index` is the index of the entry within the page table (0–1023 on x86) and
    /// `pte` is the decoded [`PageTableEntry`].
    ///
    pub fn iter_present_ptes(&self) -> impl Iterator<Item = (usize, PageTableEntry)> + '_ {
        (0..self.entries.len()).filter_map(
            // |idx| match PageTableEntry::from_raw_value(self.entries[idx]) {
            |idx| match PageTableEntry::from_raw_value(
                self.env_interaction_read_page_table_entry(idx),
            ) {
                Some(pte) if pte.is_present() => Some((idx, pte)),
                _ => None,
            },
        )
    }
}
