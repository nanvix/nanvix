// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::log::debug;
use ::sha2::{
    Digest,
    Sha256,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Computes the SHA256 hash of a byte slice.
///
/// # Parameters
///
/// - `data`: The data to hash.
///
/// # Returns
///
/// The SHA256 hash as a lowercase hexadecimal string.
///
pub(crate) fn compute_sha256(data: &[u8]) -> String {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(data);
    let result: sha2::digest::Output<Sha256> = hasher.finalize();
    hex_encode(&result)
}

///
/// # Description
///
/// Verifies that data matches an expected SHA256 checksum.
///
/// # Parameters
///
/// - `data`: The data to verify.
/// - `expected_checksum`: The expected SHA256 checksum as a hexadecimal string.
///
/// # Returns
///
/// `true` if the computed checksum matches the expected checksum, `false` otherwise.
///
pub(crate) fn verify_sha256(data: &[u8], expected_checksum: &str) -> bool {
    let computed: String = compute_sha256(data);
    let expected_normalized: String = expected_checksum.to_lowercase().trim().to_string();
    let matches: bool = computed == expected_normalized;
    if !matches {
        debug!("Checksum mismatch: expected '{}', computed '{}'", expected_normalized, computed);
    }
    matches
}

///
/// # Description
///
/// Converts a byte slice to a lowercase hexadecimal string.
///
/// # Parameters
///
/// - `bytes`: The bytes to convert.
///
/// # Returns
///
/// A lowercase hexadecimal string representation of the bytes.
///
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    ///
    /// # Description
    ///
    /// Tests SHA256 computation for known test vectors.
    ///
    #[test]
    fn test_compute_sha256_empty() {
        // SHA256 of empty string is a known value.
        let hash: String = compute_sha256(b"");
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    ///
    /// # Description
    ///
    /// Tests SHA256 computation for "hello" string.
    ///
    #[test]
    fn test_compute_sha256_hello() {
        let hash: String = compute_sha256(b"hello");
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    ///
    /// # Description
    ///
    /// Tests SHA256 verification with matching checksum.
    ///
    #[test]
    fn test_verify_sha256_match() {
        let data: &[u8] = b"hello";
        let checksum: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_sha256(data, checksum));
    }

    ///
    /// # Description
    ///
    /// Tests SHA256 verification with uppercase checksum.
    ///
    #[test]
    fn test_verify_sha256_uppercase() {
        let data: &[u8] = b"hello";
        let checksum: &str = "2CF24DBA5FB0A30E26E83B2AC5B9E29E1B161E5C1FA7425E73043362938B9824";
        assert!(verify_sha256(data, checksum));
    }

    ///
    /// # Description
    ///
    /// Tests SHA256 verification with checksum containing whitespace.
    ///
    #[test]
    fn test_verify_sha256_with_whitespace() {
        let data: &[u8] = b"hello";
        let checksum: &str =
            "  2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824  \n";
        assert!(verify_sha256(data, checksum));
    }

    ///
    /// # Description
    ///
    /// Tests SHA256 verification with mismatched checksum.
    ///
    #[test]
    fn test_verify_sha256_mismatch() {
        let data: &[u8] = b"hello";
        let wrong_checksum: &str =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(!verify_sha256(data, wrong_checksum));
    }

    ///
    /// # Description
    ///
    /// Tests hex encoding.
    ///
    #[test]
    fn test_hex_encode() {
        assert_eq!(hex_encode(&[0x00, 0xff, 0x10, 0xab]), "00ff10ab");
    }
}
