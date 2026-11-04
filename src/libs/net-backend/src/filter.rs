// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Host egress filtering for guest network connections.
//!
//! Nanvix proxies guest sockets through the host-side network daemon, so the
//! daemon is the natural enforcement point for per-host egress policy. The types
//! here carry a resolved IPv4/CIDR policy -- a default action plus allow and deny
//! rules (typically forwarded by a consumer such as MXC) -- and answer the single
//! question the daemon asks before completing a `connect()`: *is this destination
//! permitted?*

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
// InvalidHostRule
//==================================================================================================

///
/// # Description
///
/// Error returned when a host filter rule is not a valid IPv4 address or CIDR block.
///
/// Policies are built fail-closed: a single malformed rule rejects the whole policy instead of
/// being dropped, since silently skipping a deny rule would widen egress.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidHostRule {
    /// The offending rule, verbatim.
    entry: String,
}

impl InvalidHostRule {
    /// Returns the offending rule, verbatim.
    pub fn entry(&self) -> &str {
        &self.entry
    }
}

impl ::std::fmt::Display for InvalidHostRule {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "invalid host rule (expected IPv4 or CIDR): {}", self.entry)
    }
}

impl ::std::error::Error for InvalidHostRule {}

///
/// # Description
///
/// Parses every entry in `entries`, failing on the first malformed one.
///
fn parse_rules(entries: &[String]) -> Result<Vec<Ipv4Cidr>, InvalidHostRule> {
    entries
        .iter()
        .map(|entry| {
            Ipv4Cidr::parse(entry).ok_or_else(|| InvalidHostRule {
                entry: entry.clone(),
            })
        })
        .collect()
}

//==================================================================================================
// HostAction
//==================================================================================================

///
/// # Description
///
/// Verdict a [`HostFilter`] reaches for a destination. Also used as the action applied to
/// destinations that match no rule.
///
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HostAction {
    /// Permit the destination.
    #[default]
    Allow,
    /// Deny the destination.
    Deny,
}

//==================================================================================================
// HostFilter
//==================================================================================================

///
/// # Description
///
/// Host network egress policy applied to guest `connect()` / `sendto()` destinations.
///
/// A policy composes an explicit default action with simultaneous allow and deny rule
/// collections. Destinations are evaluated with fixed precedence:
///
/// 1. A destination matching any deny rule is denied.
/// 2. Otherwise, a destination matching any allow rule is permitted.
/// 3. Otherwise, if the DNS exception is enabled, a connection to the DNS port is permitted (see
///    [`HostFilter::permits_connection`]).
/// 4. Otherwise, the default action applies.
///
/// Deny always wins, so the verdict does not depend on the order in which rules were declared,
/// and neither an allow rule nor the DNS exception can reopen an explicitly denied destination.
///
/// The legacy postures map onto this model as follows:
///
/// | Posture   | Default action | Allow rules | Deny rules |
/// |-----------|----------------|-------------|------------|
/// | allow-all | allow          | none        | none       |
/// | deny-all  | deny           | none        | none       |
/// | allowlist | deny           | list        | none       |
/// | blocklist | allow          | none        | list       |
///
/// The [`Default`] policy is allow-all with the DNS exception disabled. It is used when host
/// networking is enabled with no rules and when no explicit policy has been configured.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HostFilter {
    /// Action applied to destinations that match no rule.
    default_action: HostAction,
    /// Destinations permitted unless they also match a deny rule.
    allow: Vec<Ipv4Cidr>,
    /// Destinations that are always denied.
    deny: Vec<Ipv4Cidr>,
    /// Whether connections to the DNS port are permitted when no rule matches.
    allow_dns: bool,
}

impl HostFilter {
    /// Destination port used for DNS. Connections to this port are permitted when the DNS
    /// exception is enabled and no deny rule matches (see [`HostFilter::permits_connection`]).
    const DNS_PORT: u16 = 53;

    ///
    /// # Description
    ///
    /// Builds a policy from already-parsed rules.
    ///
    /// # Parameters
    ///
    /// - `default_action`: Action applied to destinations that match no rule.
    /// - `allow`: Destinations permitted unless they also match a deny rule.
    /// - `deny`: Destinations that are always denied.
    /// - `allow_dns`: Whether to enable the DNS exception. Pass `false` unless name resolution
    ///   must work for destinations the default action would otherwise deny.
    ///
    pub fn new(
        default_action: HostAction,
        allow: Vec<Ipv4Cidr>,
        deny: Vec<Ipv4Cidr>,
        allow_dns: bool,
    ) -> Self {
        Self {
            default_action,
            allow,
            deny,
            allow_dns,
        }
    }

    /// Returns a policy that permits every destination.
    pub fn allow_all() -> Self {
        Self::default()
    }

    /// Returns a policy that denies every destination, including DNS.
    pub fn deny_all() -> Self {
        Self::new(HostAction::Deny, Vec::new(), Vec::new(), false)
    }

    ///
    /// # Description
    ///
    /// Builds a policy from textual allow and deny rules, each an IPv4 address (`a.b.c.d`,
    /// implicit `/32`) or CIDR block (`a.b.c.d/n`).
    ///
    /// See [`HostFilter::new`] for the meaning of the parameters.
    ///
    /// # Errors
    ///
    /// Every rule is validated before the policy is built. Returns [`InvalidHostRule`] for the
    /// first rule that is not a valid IPv4 address or CIDR block; no partially-built policy is
    /// returned.
    ///
    pub fn from_rules(
        default_action: HostAction,
        allow: &[String],
        deny: &[String],
        allow_dns: bool,
    ) -> Result<Self, InvalidHostRule> {
        let allow: Vec<Ipv4Cidr> = parse_rules(allow)?;
        let deny: Vec<Ipv4Cidr> = parse_rules(deny)?;
        Ok(Self::new(default_action, allow, deny, allow_dns))
    }

    ///
    /// # Description
    ///
    /// Builds a policy from legacy allow / block lists.
    ///
    /// The default action is derived from the lists: a non-empty `allow` list selects
    /// deny-by-default (allowlist posture), otherwise the default is allow (blocklist or
    /// allow-all posture). `block` entries always become deny rules, so when both lists are
    /// given the result is an allowlist narrowed by explicit denies.
    ///
    /// `exempt_dns` enables the DNS exception for the allowlist posture only; it is ignored when
    /// `allow` is empty, where the default action already permits DNS.
    ///
    /// # Errors
    ///
    /// Same as [`HostFilter::from_rules`].
    ///
    pub fn from_lists(
        allow: &[String],
        block: &[String],
        exempt_dns: bool,
    ) -> Result<Self, InvalidHostRule> {
        let allowlist: bool = !allow.is_empty();
        let default_action: HostAction = if allowlist {
            HostAction::Deny
        } else {
            HostAction::Allow
        };
        Self::from_rules(default_action, allow, block, allowlist && exempt_dns)
    }

    /// Returns the action applied to destinations that match no rule.
    pub fn default_action(&self) -> HostAction {
        self.default_action
    }

    /// Returns the allow rules.
    pub fn allow_rules(&self) -> &[Ipv4Cidr] {
        &self.allow
    }

    /// Returns the deny rules.
    pub fn deny_rules(&self) -> &[Ipv4Cidr] {
        &self.deny
    }

    /// Returns whether the DNS exception is enabled.
    pub fn allows_dns(&self) -> bool {
        self.allow_dns
    }

    ///
    /// # Description
    ///
    /// Evaluates the policy for `addr` (IPv4 octets) and, if known, the destination `port`.
    ///
    fn evaluate(&self, addr: [u8; 4], port: Option<u16>) -> HostAction {
        if self.deny.iter().any(|c| c.contains(addr)) {
            return HostAction::Deny;
        }
        let allowed_by_rule: bool = self.allow.iter().any(|c| c.contains(addr));
        let allowed_as_dns: bool = self.allow_dns && port == Some(Self::DNS_PORT);
        if allowed_by_rule || allowed_as_dns {
            HostAction::Allow
        } else {
            self.default_action
        }
    }

    /// Returns whether a connection to `addr` (IPv4 octets) is permitted, ignoring the DNS
    /// exception.
    pub fn permits(&self, addr: [u8; 4]) -> bool {
        self.evaluate(addr, None) == HostAction::Allow
    }

    ///
    /// # Description
    ///
    /// Returns whether a connection to `addr`:`port` is permitted, applying the DNS exception on
    /// top of [`HostFilter::permits`].
    ///
    /// Under a deny-by-default policy the configured resolver is usually not among the allowed
    /// destinations, yet name resolution must succeed for those destinations to be reachable.
    /// When the DNS exception is enabled, connections to the DNS port are permitted regardless
    /// of destination IP, mirroring the always-allow `:53` rule other Nanvix consumers (e.g.
    /// MXC's LXC and WSLC backends) install in allowlist mode.
    ///
    /// The exception is disabled by default and is evaluated after the deny rules, so it never
    /// overrides an explicit deny. Restricting it to specific resolver IPs is left to the
    /// caller, which can encode them as allow rules instead.
    ///
    pub fn permits_connection(&self, addr: [u8; 4], port: u16) -> bool {
        self.evaluate(addr, Some(port)) == HostAction::Allow
    }

    /// Returns `true` if this policy applies no restrictions: the default action is allow and
    /// there are no deny rules. Used to decide whether non-IPv4 destinations, which `permits`
    /// cannot evaluate, should be permitted.
    pub fn is_allow_all(&self) -> bool {
        self.default_action == HostAction::Allow && self.deny.is_empty()
    }
}

//==================================================================================================
// Unit tests
//==================================================================================================

#[cfg(test)]
mod tests;
