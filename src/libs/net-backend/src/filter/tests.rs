// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn deny_all_blocks_everything() {
    let f = HostFilter::deny_all();
    assert!(!f.permits([0, 0, 0, 0]));
    assert!(!f.permits([8, 8, 8, 8]));
    assert!(!f.is_allow_all());
}

#[test]
fn default_is_allow_all() {
    assert_eq!(HostFilter::default(), HostFilter::allow_all());
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
    let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false).unwrap();
    assert!(f.permits([1, 1, 1, 1]));
    assert!(!f.permits([8, 8, 8, 8]));
}

#[test]
fn blocklist_allows_by_default() {
    let f = HostFilter::from_lists(&[], &["8.8.8.8".to_string()], false).unwrap();
    assert!(!f.permits([8, 8, 8, 8]));
    assert!(f.permits([1, 1, 1, 1]));
}

#[test]
fn empty_is_allow_all() {
    let f = HostFilter::from_lists(&[], &[], false).unwrap();
    assert_eq!(f, HostFilter::allow_all());
    assert!(f.permits([8, 8, 8, 8]));
}

#[test]
fn is_allow_all_reflects_filtering() {
    assert!(HostFilter::allow_all().is_allow_all());
    assert!(!HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false)
        .unwrap()
        .is_allow_all());
    assert!(!HostFilter::from_lists(&[], &["8.8.8.8".to_string()], false)
        .unwrap()
        .is_allow_all());
}

#[test]
fn allowlist_exempts_dns_port_when_opted_in() {
    let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], true).unwrap();
    // A resolver outside the allowlist is reachable on port 53 only.
    assert!(f.permits_connection([8, 8, 8, 8], 53));
    assert!(!f.permits_connection([8, 8, 8, 8], 443));
    // Allowed hosts remain reachable on any port.
    assert!(f.permits_connection([1, 1, 1, 1], 443));
}

#[test]
fn strict_allowlist_denies_dns_port() {
    // Without the opt-in carve-out, even :53 must match an allowlist block.
    let f = HostFilter::from_lists(&["1.1.1.1".to_string()], &[], false).unwrap();
    assert!(!f.permits_connection([8, 8, 8, 8], 53));
    assert!(f.permits_connection([1, 1, 1, 1], 53));
}

#[test]
fn dns_exemption_scoped_to_allowlist() {
    // Deny-all stays fully closed, including DNS.
    assert!(!HostFilter::deny_all().permits_connection([8, 8, 8, 8], 53));
    // Blocklist never has the carve-out override an explicit block.
    let f = HostFilter::from_lists(&[], &["8.8.8.8".to_string()], true).unwrap();
    assert!(!f.permits_connection([8, 8, 8, 8], 53));
    assert!(f.permits_connection([1, 1, 1, 1], 53));
}

#[test]
fn from_lists_rejects_malformed_allow_rule() {
    let err = HostFilter::from_lists(&["1.1.1.1".to_string(), "nope".to_string()], &[], false)
        .expect_err("malformed allow rule must be rejected");
    assert_eq!(err.entry(), "nope");
}

#[test]
fn from_lists_rejects_malformed_block_rule() {
    // A dropped block rule would widen egress, so the whole policy must fail.
    let err =
        HostFilter::from_lists(&[], &["8.8.8.8".to_string(), "8.8.4.4/33".to_string()], false)
            .expect_err("malformed block rule must be rejected");
    assert_eq!(err.entry(), "8.8.4.4/33");
}

#[test]
fn from_lists_validates_block_rules_alongside_allow_rules() {
    // Block rules are validated even when the allowlist would take precedence.
    let res = HostFilter::from_lists(&["1.1.1.1".to_string()], &["bad".to_string()], false);
    assert_eq!(
        res.expect_err("malformed block rule must be rejected")
            .entry(),
        "bad"
    );
}

#[test]
fn from_lists_composes_allowlist_with_block_rules() {
    let f = HostFilter::from_lists(&["10.0.0.0/8".to_string()], &["10.0.0.1".to_string()], true)
        .unwrap();
    assert_eq!(f.default_action(), HostAction::Deny);
    // The block rule narrows the allowlist instead of being ignored.
    assert!(!f.permits([10, 0, 0, 1]));
    assert!(f.permits([10, 0, 0, 2]));
    assert!(!f.permits([8, 8, 8, 8]));
    // The DNS exception never reopens an explicitly denied destination.
    assert!(!f.permits_connection([10, 0, 0, 1], 53));
    assert!(f.permits_connection([8, 8, 8, 8], 53));
}

//==================================================================================================
// Composed policy
//==================================================================================================

/// Converts string literals into owned rule entries.
fn rules(entries: &[&str]) -> Vec<String> {
    entries.iter().map(|e| e.to_string()).collect()
}

/// Builds a validated policy from string-literal rules.
fn policy(
    default_action: HostAction,
    allow: &[&str],
    deny: &[&str],
    allow_dns: bool,
) -> HostFilter {
    HostFilter::from_rules(default_action, &rules(allow), &rules(deny), allow_dns).unwrap()
}

#[test]
fn empty_rules_follow_default_action() {
    let open = policy(HostAction::Allow, &[], &[], false);
    assert!(open.permits([8, 8, 8, 8]));
    assert!(open.is_allow_all());
    let closed = policy(HostAction::Deny, &[], &[], false);
    assert!(!closed.permits([8, 8, 8, 8]));
    assert!(!closed.is_allow_all());
    assert_eq!(closed, HostFilter::deny_all());
}

#[test]
fn combined_rules_fall_back_to_default_action() {
    for default_action in [HostAction::Allow, HostAction::Deny] {
        let f = policy(default_action, &["10.0.0.0/8"], &["10.1.0.0/16"], false);
        assert!(f.permits([10, 2, 0, 1]));
        assert!(!f.permits([10, 1, 0, 1]));
        // Unmatched traffic follows the explicit default action.
        assert_eq!(f.permits([8, 8, 8, 8]), default_action == HostAction::Allow);
        // Deny rules always restrict, so non-IPv4 destinations are never waved through.
        assert!(!f.is_allow_all());
    }
}

#[test]
fn deny_wins_on_overlap_regardless_of_order() {
    let forward = policy(HostAction::Allow, &["1.1.1.1", "10.0.0.0/8"], &["10.0.0.0/8"], false);
    let reverse = policy(HostAction::Allow, &["10.0.0.0/8", "1.1.1.1"], &["10.0.0.0/8"], false);
    for f in [&forward, &reverse] {
        assert!(!f.permits([10, 0, 0, 1]));
        assert!(f.permits([1, 1, 1, 1]));
    }
    // A narrow deny carves a hole out of a broad allow, and a narrow allow cannot reopen a
    // broad deny.
    let narrow_deny = policy(HostAction::Deny, &["10.0.0.0/8"], &["10.0.0.1"], false);
    assert!(!narrow_deny.permits([10, 0, 0, 1]));
    assert!(narrow_deny.permits([10, 0, 0, 2]));
    let broad_deny = policy(HostAction::Allow, &["10.0.0.1"], &["10.0.0.0/8"], false);
    assert!(!broad_deny.permits([10, 0, 0, 1]));
}

#[test]
fn duplicate_rules_do_not_change_verdicts() {
    let single = policy(HostAction::Deny, &["1.1.1.1"], &["10.0.0.1"], false);
    let duplicated =
        policy(HostAction::Deny, &["1.1.1.1", "1.1.1.1/32"], &["10.0.0.1", "10.0.0.1"], false);
    for addr in [[1, 1, 1, 1], [10, 0, 0, 1], [8, 8, 8, 8]] {
        assert_eq!(single.permits(addr), duplicated.permits(addr));
    }
}

#[test]
fn zero_prefix_matches_every_address() {
    let allow_any = policy(HostAction::Deny, &["0.0.0.0/0"], &["8.8.8.8"], false);
    assert!(allow_any.permits([255, 255, 255, 255]));
    assert!(allow_any.permits([0, 0, 0, 0]));
    assert!(!allow_any.permits([8, 8, 8, 8]));
    // A /0 deny closes every IPv4 destination, even explicitly allowed ones and DNS.
    let deny_any = policy(HostAction::Allow, &["1.1.1.1"], &["0.0.0.0/0"], true);
    assert!(!deny_any.permits([1, 1, 1, 1]));
    assert!(!deny_any.permits_connection([8, 8, 8, 8], 53));
    assert!(!deny_any.is_allow_all());
}

#[test]
fn dns_exception_is_disabled_by_default() {
    assert!(!HostFilter::default().allows_dns());
    assert!(!HostFilter::allow_all().allows_dns());
    assert!(!HostFilter::deny_all().allows_dns());
    assert!(
        !policy(HostAction::Deny, &["1.1.1.1"], &[], false).permits_connection([8, 8, 8, 8], 53)
    );
}

#[test]
fn dns_exception_permits_only_dns_port() {
    let f = policy(HostAction::Deny, &[], &[], true);
    assert!(f.permits_connection([8, 8, 8, 8], 53));
    assert!(!f.permits_connection([8, 8, 8, 8], 443));
    // permits() has no port, so it never applies the exception.
    assert!(!f.permits([8, 8, 8, 8]));
}

#[test]
fn dns_exception_never_overrides_deny_rule() {
    let f = policy(HostAction::Deny, &["8.8.8.8"], &["8.8.8.8"], true);
    assert!(!f.permits_connection([8, 8, 8, 8], 53));
    assert!(f.permits_connection([8, 8, 4, 4], 53));
}

#[test]
fn from_rules_rejects_malformed_rules() {
    let err = HostFilter::from_rules(HostAction::Deny, &rules(&["1.1.1.1"]), &rules(&["x"]), false)
        .expect_err("malformed deny rule must be rejected");
    assert_eq!(err.entry(), "x");
    let err = HostFilter::from_rules(HostAction::Allow, &rules(&["1.1.1.1/-1"]), &[], false)
        .expect_err("malformed allow rule must be rejected");
    assert_eq!(err.entry(), "1.1.1.1/-1");
}

#[test]
fn from_lists_translates_legacy_postures() {
    let allowlist = HostFilter::from_lists(&rules(&["1.1.1.1"]), &[], true).unwrap();
    assert_eq!(allowlist, policy(HostAction::Deny, &["1.1.1.1"], &[], true));
    let blocklist = HostFilter::from_lists(&[], &rules(&["8.8.8.8"]), true).unwrap();
    assert_eq!(blocklist, policy(HostAction::Allow, &[], &["8.8.8.8"], false));
    assert_eq!(HostFilter::from_lists(&[], &[], true).unwrap(), HostFilter::allow_all());
}
