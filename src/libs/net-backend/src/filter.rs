// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host egress filtering for guest network connections.
//!
//! Nanvix proxies guest sockets through the host-side network daemon, so the
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
/// `AllowAll` is used when host networking is enabled with no per-host list and
/// is the default when no explicit policy has been configured; `Allow`/`Block`
/// carry the resolved IPv4/CIDR set.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HostFilter {
    /// No filtering: every destination is permitted. Used when host networking
    /// is enabled with no per-host list, and the default when no explicit policy
    /// has been configured.
    #[default]
    AllowAll,
    /// Deny all: every destination is blocked. Must be selected explicitly by a
    /// caller that wants a fully-closed policy.
    DenyAll,
    /// Allowlist: only destinations matching one of these blocks are permitted.
    ///
    /// `exempt_dns` opts into a DNS carve-out that additionally permits any
    /// destination on the DNS port (see [`HostFilter::permits_connection`]).
    /// When `false`, the allowlist is strict and even `:53` must match a block.
    Allow {
        /// Allowed IPv4/CIDR blocks.
        cidrs: Vec<Ipv4Cidr>,
        /// Whether the DNS carve-out is enabled for this allowlist.
        exempt_dns: bool,
    },
    /// Blocklist: every destination is permitted except those matching a block.
    Block(Vec<Ipv4Cidr>),
}

impl HostFilter {
    /// Destination port used for DNS. Connections to this port are exempted from
    /// the allowlist when the carve-out is opted into (see
    /// [`HostFilter::permits_connection`]).
    const DNS_PORT: u16 = 53;

    ///
    /// # Description
    ///
    /// Builds a filter from the allow / block entry lists.
    ///
    /// `allow` takes precedence over `block`; if both are empty, returns
    /// [`HostFilter::AllowAll`]. Unparsable entries are skipped — callers are
    /// expected to validate entries upstream.
    ///
    /// `exempt_dns` only applies in allowlist mode and opts into the DNS
    /// carve-out (see [`HostFilter::permits_connection`]); pass `false` for a
    /// strict allowlist. It is a no-op for block / allow-all results.
    ///
    pub fn from_lists(allow: &[String], block: &[String], exempt_dns: bool) -> Self {
        if !allow.is_empty() {
            Self::Allow {
                cidrs: allow.iter().filter_map(|e| Ipv4Cidr::parse(e)).collect(),
                exempt_dns,
            }
        } else if !block.is_empty() {
            Self::Block(block.iter().filter_map(|e| Ipv4Cidr::parse(e)).collect())
        } else {
            Self::AllowAll
        }
    }

    /// Returns whether a connection to `addr` (IPv4 octets) is permitted.
    pub fn permits(&self, addr: [u8; 4]) -> bool {
        match self {
            Self::DenyAll => false,
            Self::AllowAll => true,
            Self::Allow { cidrs, .. } => cidrs.iter().any(|c| c.contains(addr)),
            Self::Block(list) => !list.iter().any(|c| c.contains(addr)),
        }
    }

    ///
    /// # Description
    ///
    /// Returns whether a connection to `addr`:`port` is permitted, applying an
    /// opt-in DNS carve-out on top of [`HostFilter::permits`].
    ///
    /// In allowlist mode ([`HostFilter::Allow`]) the configured resolver is
    /// usually not among the allowed destinations, yet name resolution must
    /// succeed for those hosts to be reachable. When the allowlist opts into the
    /// carve-out (`exempt_dns == true`), connections to the DNS port are
    /// permitted regardless of destination IP, mirroring the always-allow `:53`
    /// rule other Nanvix consumers (e.g. MXC's LXC and WSLC backends) install in
    /// allowlist mode.
    ///
    /// The carve-out is opt-in and scoped to allowlist mode only: a strict
    /// allowlist (`exempt_dns == false`) still requires `:53` to match a block,
    /// it never relaxes [`HostFilter::DenyAll`] (networking off stays fully
    /// closed), and it never overrides an explicit block in
    /// [`HostFilter::Block`] mode. Restricting the exemption to specific resolver
    /// IPs is left to the caller, which can encode them as allowlist blocks.
    ///
    pub fn permits_connection(&self, addr: [u8; 4], port: u16) -> bool {
        if port == Self::DNS_PORT
            && matches!(
                self,
                Self::Allow {
                    exempt_dns: true,
                    ..
                }
            )
        {
            return true;
        }
        self.permits(addr)
    }

    /// Returns `true` if this filter applies no restrictions (i.e. is
    /// [`HostFilter::AllowAll`]). Used to decide whether non-IPv4 destinations,
    /// which `permits` cannot evaluate, should be permitted.
    pub fn is_allow_all(&self) -> bool {
        matches!(self, Self::AllowAll)
    }
}

//==================================================================================================
// Unit tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_all_blocks_everything() {
        let f = HostFilter::DenyAll;
        assert!(!f.permits([0, 0, 0, 0]));
        assert!(!f.permits([8, 8, 8, 8]));
        assert!(!f.is_allow_all());
    }

    #[test]
    fn default_is_allow_all() {
        assert!(matches!(HostFilter::default(), HostFilter::AllowAll));
    }

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
        let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false);
        assert!(f.permits([1, 1, 1, 1]));
        assert!(!f.permits([8, 8, 8, 8]));
    }

    #[test]
    fn blocklist_allows_by_default() {
        let f = HostFilter::from_lists(&[], &["8.8.8.8".to_string()], false);
        assert!(!f.permits([8, 8, 8, 8]));
        assert!(f.permits([1, 1, 1, 1]));
    }

    #[test]
    fn empty_is_allow_all() {
        let f = HostFilter::from_lists(&[], &[], false);
        assert!(matches!(f, HostFilter::AllowAll));
        assert!(f.permits([8, 8, 8, 8]));
    }

    #[test]
    fn is_allow_all_reflects_filtering() {
        assert!(HostFilter::AllowAll.is_allow_all());
        assert!(!HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false).is_allow_all());
        assert!(!HostFilter::from_lists(&[], &["8.8.8.8".to_string()], false).is_allow_all());
    }

    #[test]
    fn allowlist_exempts_dns_port_when_opted_in() {
        let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], true);
        // A resolver outside the allowlist is reachable on port 53 only.
        assert!(f.permits_connection([8, 8, 8, 8], 53));
        assert!(!f.permits_connection([8, 8, 8, 8], 443));
        // Allowed hosts remain reachable on any port.
        assert!(f.permits_connection([1, 1, 1, 1], 443));
    }

    #[test]
    fn strict_allowlist_denies_dns_port() {
        // Without the opt-in carve-out, even :53 must match an allowlist block.
        let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false);
        assert!(!f.permits_connection([8, 8, 8, 8], 53));
        assert!(f.permits_connection([1, 1, 1, 1], 53));
    }

    #[test]
    fn dns_exemption_scoped_to_allowlist() {
        // DenyAll stays fully closed, including DNS.
        assert!(!HostFilter::DenyAll.permits_connection([8, 8, 8, 8], 53));
        // Blocklist never has the carve-out override an explicit block.
        let f = HostFilter::from_lists(&[], &["8.8.8.8".to_string()], true);
        assert!(!f.permits_connection([8, 8, 8, 8], 53));
        assert!(f.permits_connection([1, 1, 1, 1], 53));
    }
}
