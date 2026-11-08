// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Iterator over the pieces of a string that are at most a given number of bytes long. Pieces end
/// at character boundaries, so each one is a valid string on its own, and each piece but the last
/// is as long as possible.
///
/// A character that is longer than the maximum length of a piece is yielded whole, so that
/// iteration always makes progress.
///
pub(crate) struct Chunks<'a> {
    /// Part of the string that was not yielded yet.
    remaining: &'a str,
    /// Maximum length of a piece (in bytes).
    max_len: usize,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl<'a> Chunks<'a> {
    ///
    /// # Description
    ///
    /// Creates an iterator over the pieces of a string.
    ///
    /// # Parameters
    ///
    /// - `s`: String to split.
    /// - `max_len`: Maximum length of a piece (in bytes).
    ///
    /// # Returns
    ///
    /// An iterator over the pieces of `s`.
    ///
    pub(crate) fn new(s: &'a str, max_len: usize) -> Self {
        Self {
            remaining: s,
            max_len,
        }
    }
}

impl<'a> Iterator for Chunks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        let mut end: usize = self.remaining.floor_char_boundary(self.max_len);
        // The first character does not fit in a piece: yield it whole.
        if end == 0 {
            end = self.remaining.ceil_char_boundary(1);
        }

        let (chunk, remaining): (&'a str, &'a str) = self.remaining.split_at_checked(end)?;
        self.remaining = remaining;
        Some(chunk)
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod tests;
