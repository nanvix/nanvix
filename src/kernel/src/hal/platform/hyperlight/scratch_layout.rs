// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Macros
//==================================================================================================

///
/// # Description
///
/// Declarative macro that computes a compile-time layout for the scratch-reserved region.
///
/// Given a `page_align` expression and an ordered list of `(NAME, size_expr, align_expr)` entries
/// the macro emits:
///
///  - `const {NAME}_SIZE: usize`         — size of each entry (equal to `size_expr`).
///  - `const {NAME}_OFFSET: usize`       — byte offset from the scratch-reserved base, aligned
///    to `align_expr`.
///  - `unsafe fn {name}_ptr(base: usize) -> *mut u8` — accessor that returns a pointer to the
///    entry at the given base address.
///  - `const SCRATCH_RESERVED_SIZE: usize` — page-aligned total size covering all entries.
///
/// # Example
///
/// ```ignore
/// scratch_layout! {
///     page_align = PAGE_ALIGNMENT;
///
///     FRAME_ALLOC_BITMAP : size = FRAME_ALLOCATOR_STORAGE_SIZE, align = WORD_ALIGNMENT;
///     KPOOL_BITMAP       : size = KPOOL_BITMAP_STORAGE_SIZE,    align = WORD_ALIGNMENT;
///     GDT                : size = GDT_STORAGE_SIZE,             align = gdt::GDTE_ALIGNMENT;
/// }
/// ```
///
macro_rules! scratch_layout {
    // Public entry point: capture the page alignment and kick off the recursive accumulator.
    (
        page_align = $page_align:expr;

        $(#[$first_meta:meta])* $first_name:ident : size = $first_size:expr, align = $first_align:expr;
        $($(#[$rest_meta:meta])* $rest_name:ident : size = $rest_size:expr, align = $rest_align:expr;)*
    ) => {
        scratch_layout!(
            @step 0usize; page_align = $page_align;
            $(#[$first_meta])* $first_name : size = $first_size, align = $first_align;
            $($(#[$rest_meta])* $rest_name : size = $rest_size, align = $rest_align;)*
        );
    };

    // Recursive step: emit constants and accessor for one entry, then recurse.
    (
        @step $acc:expr; page_align = $page_align:expr;
        $(#[$meta:meta])* $name:ident : size = $size:expr, align = $align:expr;
        $($(#[$rest_meta:meta])* $rest_name:ident : size = $rest_size:expr, align = $rest_align:expr;)*
    ) => {
        ::paste::paste! {
            $(#[$meta])*
            /// Size in bytes of the scratch-resident storage for this entry.
            const [<$name _SIZE>]: usize = $size;

            /// Byte offset of this entry from the scratch-reserved base, aligned to the
            /// entry's required alignment.
            const [<$name _OFFSET>]: usize = {
                match ::sys::mm::align_up($acc, $align) {
                    Some(v) => v,
                    None => panic!(concat!("scratch_layout: alignment overflow for ", stringify!($name))),
                }
            };

            /// Returns a raw pointer to this entry given the scratch-reserved base address.
            ///
            /// # Safety
            ///
            /// `base` must be the start address of a mapped, writable region that contains
            /// at least [`SCRATCH_RESERVED_SIZE`] bytes.
            #[allow(dead_code)]
            unsafe fn [<$name:lower _ptr>](base: usize) -> *mut u8 {
                (base + [<$name _OFFSET>]) as *mut u8
            }
        }

        // Recurse for remaining entries. The new accumulator is the end of this entry.
        ::paste::paste! {
            scratch_layout!(
                @step [<$name _OFFSET>] + [<$name _SIZE>]; page_align = $page_align;
                $($(#[$rest_meta])* $rest_name : size = $rest_size, align = $rest_align;)*
            );
        }
    };

    // Terminal case: no more entries — emit the page-aligned total size.
    (@step $acc:expr; page_align = $page_align:expr;) => {
        /// Combined size of all scratch-resident kernel structures with inter-area padding,
        /// rounded up to a page boundary.
        pub(super) const SCRATCH_RESERVED_SIZE: usize = {
            let raw: usize = $acc;
            match ::sys::mm::align_up(raw, $page_align) {
                Some(v) => v,
                None => panic!("scratch_layout: page-alignment overflow for SCRATCH_RESERVED_SIZE"),
            }
        };
    };
}
