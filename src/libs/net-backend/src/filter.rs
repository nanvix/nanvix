// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host egress filtering for guest network connections.
//!
//! NanVix proxies guest sockets through the host-side network daemon, so the
//! daemon is the natural enforcement point for per-host egress policy. The types
//! here carry a resolved IPv4/CIDR allow- or block-set (typically forwarded by a
//! consumer such as MXC) and answer the single question the daemon asks before
//! completing a `connect()`: *is this destination permitted?*

//==================================================================================================
// Ipv4Cidr
//==================================================================================================

///
/// # Description
///
/// A single IPv4 address or CIDR block.
///
/// Stored as a network-order base address and mask so membership tests reduce to
/// a pair of bitwise operations.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Cidr {
    /// Network address (already masked).
    base: u32,
    /// Prefix mask (e.g. `/24` -> `0xffff_ff00`).
    mask: u32,
}

impl Ipv4Cidr {
    ///
    /// # Description
    ///
    /// Parses an entry of the form `a.b.c.d` (a single host, implicit `/32`) or
    /// `a.b.c.d/n` (a CIDR block).
    ///
    /// # Returns
    ///
    /// The parsed block, or `None` if the address is malformed or the prefix is
    /// outside `0..=32`.
    ///
    pub fn parse(entry: &str) -> Option<Self> {
        let (addr_str, prefix): (&str, u8) = match entry.trim().split_once('/') {
            Some((addr, pfx)) => {
                let p: u8 = pfx.trim().parse().ok()?;
                if p > 32 {
                    return None;
                }
                (addr.trim(), p)
            },
            None => (entry.trim(), 32u8),
        };
        let addr: ::std::net::Ipv4Addr = addr_str.parse().ok()?;
        let base: u32 = u32::from(addr);
        let mask: u32 = if prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        Some(Self {
            base: base & mask,
            mask,
        })
    }

    /// Returns whether `addr` (network-order octets) falls within this block.
    pub fn contains(&self, addr: [u8; 4]) -> bool {
        let ip: u32 = u32::from_be_bytes(addr);
        (ip & self.mask) == self.base
    }
}

//==================================================================================================
// HostFilter
//==================================================================================================

///
/// # Description
///
/// Host network egress filter applied to guest `connect()` destinations.
///
/// `AllowAll` is used when host networking is enabled with no per-host list;
/// `Allow`/`Block` carry the resolved IPv4/CIDR set.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HostFilter {
    /// No filtering: every destination is permitted.
    #[default]
    AllowAll,
    /// Allowlist: only destinations matching one of these blocks are permitted.
    Allow(Vec<Ipv4Cidr>),
    /// Blocklist: every destination is permitted except those matching a block.
    Block(Vec<Ipv4Cidr>),
}

impl HostFilter {
    ///
    /// # Description
    ///
    /// Builds a filter from the allow / block entry lists.
    ///
    /// `allow` takes precedence over `block`; if both are empty, returns
    /// [`HostFilter::AllowAll`]. Unparsable entries are skipped — callers are
    /// expected to validate entries upstream.
    ///
    pub fn from_lists(allow: &[String], block: &[String]) -> Self {
        if !allow.is_empty() {
            Self::Allow(allow.iter().filter_map(|e| Ipv4Cidr::parse(e)).collect())
        } else if !block.is_empty() {
            Self::Block(block.iter().filter_map(|e| Ipv4Cidr::parse(e)).collect())
        } else {
            Self::AllowAll
        }
    }

    /// Returns whether a connection to `addr` (IPv4 octets) is permitted.
    pub fn permits(&self, addr: [u8; 4]) -> bool {
        match self {
            Self::AllowAll => true,
            Self::Allow(list) => list.iter().any(|c| c.contains(addr)),
            Self::Block(list) => !list.iter().any(|c| c.contains(addr)),
        }
    }

    /// Returns whether this filter restricts any traffic (i.e. is not
    /// [`HostFilter::AllowAll`]). Used to decide whether non-IPv4 destinations,
    /// which `permits` cannot evaluate, must be denied.
    pub fn is_active(&self) -> bool {
        !matches!(self, Self::AllowAll)
    }
}

//==================================================================================================
// Unit tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cidr_host_match() {
        let c = Ipv4Cidr::parse("192.168.1.10").unwrap();
        assert!(c.contains([192, 168, 1, 10]));
        assert!(!c.contains([192, 168, 1, 11]));
    }

    #[test]
    fn cidr_block_match() {
        let c = Ipv4Cidr::parse("10.0.0.0/8").unwrap();
        assert!(c.contains([10, 1, 2, 3]));
        assert!(!c.contains([11, 0, 0, 1]));
    }

    #[test]
    fn cidr_rejects_bad_input() {
        assert!(Ipv4Cidr::parse("nope").is_none());
        assert!(Ipv4Cidr::parse("1.2.3.4/33").is_none());
        assert!(Ipv4Cidr::parse("256.0.0.1").is_none());
    }

    #[test]
    fn allowlist_denies_by_default() {
        let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[]);
        assert!(f.permits([1, 1, 1, 1]));
        assert!(!f.permits([8, 8, 8, 8]));
    }

    #[test]
    fn blocklist_allows_by_default() {
        let f = HostFilter::from_lists(&[], &["8.8.8.8".to_string()]);
        assert!(!f.permits([8, 8, 8, 8]));
        assert!(f.permits([1, 1, 1, 1]));
    }

    #[test]
    fn empty_is_allow_all() {
        let f = HostFilter::from_lists(&[], &[]);
        assert!(matches!(f, HostFilter::AllowAll));
        assert!(f.permits([8, 8, 8, 8]));
    }

    #[test]
    fn is_active_reflects_filtering() {
        assert!(!HostFilter::AllowAll.is_active());
        assert!(HostFilter::from_lists(&["1.1.1.1".to_string()], &[]).is_active());
        assert!(HostFilter::from_lists(&[], &["8.8.8.8".to_string()]).is_active());
    }
}
