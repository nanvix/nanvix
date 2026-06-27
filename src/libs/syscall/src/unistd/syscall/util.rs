// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_SIZE;
use ::core::cmp;
use ::sys::ipc::{
    SG_BULK_MAX_BYTES,
    SG_BULK_MAX_SEGMENTS,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the number of bytes that can be represented by a single scatter/gather bulk transfer.
///
/// # Parameters
///
/// - `ptr`: Start address of the buffer.
/// - `remaining`: Total number of bytes remaining to transfer.
///
/// # Returns
///
/// The number of bytes that fit within the scatter/gather limits.
///
pub fn sg_chunk_size(ptr: usize, remaining: usize) -> usize {
    if remaining == 0 {
        return 0;
    }

    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let first_page_bytes: usize = PAGE_SIZE - page_offset;
    let max_by_segments: usize = if SG_BULK_MAX_SEGMENTS == 0 {
        0
    } else {
        first_page_bytes + (SG_BULK_MAX_SEGMENTS as usize - 1) * PAGE_SIZE
    };

    cmp::min(remaining, cmp::min(SG_BULK_MAX_BYTES, max_by_segments))
}

///
/// # Description
///
/// Computes the number of bytes that can be transferred starting at `ptr` without crossing a page
/// boundary.
///
/// Unlike [`sg_chunk_size`], this caps a chunk at a single page. The read path uses it because the
/// IKC backends deliver at most one page per request: vfsd reads into a one-page bulk buffer, and
/// the standalone stdin bridge returns only the bytes currently available. With page-sized requests
/// a short reply unambiguously signals end-of-input -- true EOF for files, "no more bytes ready"
/// for streams -- so the read loop can stop without truncating a multi-page file or blocking a
/// partially-filled stream. A multi-page request would instead be capped by the backend, and the
/// resulting short reply would be indistinguishable from EOF. The write path has no such ambiguity
/// (the whole source buffer is already available, so it loops until no forward progress is made)
/// and therefore keeps using [`sg_chunk_size`].
///
/// # Parameters
///
/// - `ptr`: Start address of the buffer.
/// - `remaining`: Total number of bytes remaining to transfer.
///
/// # Returns
///
/// The number of bytes that fit within the current page.
///
pub fn page_chunk_size(ptr: usize, remaining: usize) -> usize {
    let page_offset: usize = ptr & (PAGE_SIZE - 1);
    let available: usize = PAGE_SIZE - page_offset;
    cmp::min(available, remaining)
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Number of distinct pages a `len`-byte buffer starting at `ptr` touches. This is the upper
    /// bound on the number of scatter/gather segment descriptors the kernel builds for the chunk,
    /// because the kernel emits at most one descriptor per page it spans (contiguous physical pages
    /// are merged, so the real count is never larger).
    fn pages_spanned(ptr: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let page_offset: usize = ptr & (PAGE_SIZE - 1);
        (page_offset + len).div_ceil(PAGE_SIZE)
    }

    /// A zero-length request transfers nothing.
    #[test]
    fn zero_remaining_yields_zero() {
        assert_eq!(sg_chunk_size(0, 0), 0, "an empty request must produce an empty chunk");
        assert_eq!(
            sg_chunk_size(PAGE_SIZE - 1, 0),
            0,
            "an empty request must produce an empty chunk regardless of alignment"
        );
    }

    /// A buffer that already fits within the limits is transferred whole.
    #[test]
    fn small_buffer_is_not_split() {
        assert_eq!(sg_chunk_size(0, 100), 100, "a small page-aligned buffer must not be split");
        assert_eq!(
            sg_chunk_size(PAGE_SIZE - 10, 5),
            5,
            "a small buffer within a single page must not be split"
        );
    }

    /// A page-aligned buffer larger than the limit is capped at the maximum transfer size.
    #[test]
    fn page_aligned_large_buffer_caps_at_max_bytes() {
        assert_eq!(
            sg_chunk_size(0, SG_BULK_MAX_BYTES * 4),
            SG_BULK_MAX_BYTES,
            "a page-aligned buffer must be capped at the maximum scatter/gather transfer size"
        );
    }

    /// The chunk size must never describe more pages than the kernel can turn into descriptors in a
    /// single bounded heap allocation. This is the invariant that keeps the kernel's descriptor
    /// list within one heap slab; violating it reintroduces an unbounded allocation that the
    /// kernel-heap allocator rejects.
    #[test]
    fn chunk_never_exceeds_segment_budget() {
        let offsets: [usize; 6] = [0, 1, 37, PAGE_SIZE / 2, PAGE_SIZE - 100, PAGE_SIZE - 1];
        let remainings: [usize; 5] = [
            1,
            PAGE_SIZE,
            SG_BULK_MAX_BYTES - 1,
            SG_BULK_MAX_BYTES,
            SG_BULK_MAX_BYTES * 3,
        ];

        for &offset in &offsets {
            for &remaining in &remainings {
                let chunk: usize = sg_chunk_size(offset, remaining);

                assert!(
                    chunk > 0,
                    "a non-empty request must make progress (offset={offset}, \
                     remaining={remaining})"
                );
                assert!(
                    chunk <= remaining,
                    "a chunk must never exceed the requested length (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                assert!(
                    chunk <= SG_BULK_MAX_BYTES,
                    "a chunk must never exceed the maximum transfer size (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                assert!(
                    pages_spanned(offset, chunk) <= SG_BULK_MAX_SEGMENTS as usize,
                    "a chunk must never span more pages than the segment budget (offset={offset}, \
                     remaining={remaining}, chunk={chunk}, pages={}, max={SG_BULK_MAX_SEGMENTS})",
                    pages_spanned(offset, chunk)
                );
            }
        }
    }

    /// A `page_chunk_size` request that stays within a single page is transferred whole.
    #[test]
    fn page_chunk_within_single_page_is_not_split() {
        assert_eq!(
            page_chunk_size(0, 100),
            100,
            "a small page-aligned buffer must be transferred whole"
        );
        assert_eq!(
            page_chunk_size(PAGE_SIZE - 10, 5),
            5,
            "a small buffer that stays within one page must be transferred whole"
        );
    }

    /// A `page_chunk_size` request is capped at the end of the page that holds its first byte, so a
    /// chunk never crosses a page boundary. This is the property that lets the read loop treat a
    /// short reply as end-of-input rather than truncating a multi-page transfer.
    #[test]
    fn page_chunk_stops_at_page_boundary() {
        assert_eq!(
            page_chunk_size(0, PAGE_SIZE * 4),
            PAGE_SIZE,
            "a page-aligned multi-page request must be capped at a single page"
        );
        assert_eq!(
            page_chunk_size(PAGE_SIZE - 100, PAGE_SIZE * 4),
            100,
            "an unaligned request must be capped at the bytes left in its first page"
        );
    }

    /// Whatever the alignment and length, a `page_chunk_size` chunk must make forward progress and
    /// must never straddle a page boundary.
    #[test]
    fn page_chunk_never_crosses_a_page() {
        let offsets: [usize; 6] = [0, 1, 37, PAGE_SIZE / 2, PAGE_SIZE - 100, PAGE_SIZE - 1];
        let remainings: [usize; 5] = [1, 100, PAGE_SIZE - 1, PAGE_SIZE, PAGE_SIZE * 3 + 7];

        for &offset in &offsets {
            for &remaining in &remainings {
                let chunk: usize = page_chunk_size(offset, remaining);

                assert!(
                    chunk > 0,
                    "a non-empty request must make progress (offset={offset}, \
                     remaining={remaining})"
                );
                assert!(
                    chunk <= remaining,
                    "a chunk must never exceed the requested length (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
                let page_offset: usize = offset & (PAGE_SIZE - 1);
                assert!(
                    page_offset + chunk <= PAGE_SIZE,
                    "a chunk must never cross a page boundary (offset={offset}, \
                     remaining={remaining}, chunk={chunk})"
                );
            }
        }
    }
}
