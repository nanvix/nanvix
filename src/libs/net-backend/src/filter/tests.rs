// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
