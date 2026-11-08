// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Length of the longest character in UTF-8 (in bytes).
const MAX_CHAR_LEN: usize = 4;

/// Splits a string into pieces of at most `max_len` bytes.
fn split(s: &str, max_len: usize) -> Vec<&str> {
    Chunks::new(s, max_len).collect()
}

/// An empty string has no pieces.
#[test]
fn empty_string_yields_no_piece() {
    assert!(split("", MAX_CHAR_LEN).is_empty(), "an empty string must not yield any piece");
}

/// A string that fits in a piece is yielded whole.
#[test]
fn short_string_yields_one_piece() {
    assert_eq!(split("abc", 4), ["abc"]);
    assert_eq!(split("abcd", 4), ["abcd"]);
}

/// A string that does not fit in a piece is split at the maximum length.
#[test]
fn long_string_is_split_at_max_len() {
    assert_eq!(split("abcdefghij", 4), ["abcd", "efgh", "ij"]);
}

/// A piece ends before a character that does not fit in it, rather than in the middle of it.
#[test]
fn piece_ends_at_character_boundary() {
    // The euro sign takes three bytes, and only two of them fit in the first piece.
    assert_eq!(split("ab\u{20AC}cd", 4), ["ab", "\u{20AC}c", "d"]);
}

/// A character that is longer than the maximum length of a piece is yielded whole, so that
/// iteration makes progress.
#[test]
fn character_longer_than_max_len_is_yielded_whole() {
    // The grinning face takes four bytes.
    assert_eq!(split("\u{1F600}a", 2), ["\u{1F600}", "a"]);
    assert_eq!(split("ab", 0), ["a", "b"]);
}

/// The pieces of a string with characters of every length rebuild the string, and each piece but
/// the last is as long as possible without splitting a character.
#[test]
fn pieces_are_maximal_and_rebuild_string() {
    // Characters that take one, two, three, and four bytes.
    let s: &str = "a\u{E9}b\u{20AC}c\u{1F600}d\u{E9}\u{20AC}\u{1F600}";
    for max_len in MAX_CHAR_LEN..=s.len() + 1 {
        let pieces: Vec<&str> = split(s, max_len);
        assert_eq!(pieces.concat(), s, "pieces must rebuild the string (max_len={max_len})");
        for (i, piece) in pieces.iter().enumerate() {
            assert!(!piece.is_empty(), "piece {i} is empty (max_len={max_len})");
            assert!(piece.len() <= max_len, "piece {i} is too long (max_len={max_len})");
            if let Some(next) = pieces.get(i + 1) {
                let next_len: usize = next.chars().next().map_or(0, char::len_utf8);
                assert!(
                    piece.len() + next_len > max_len,
                    "piece {i} could take the next character (max_len={max_len})"
                );
            }
        }
    }
}
