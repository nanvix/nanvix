// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::build_globset_internal;
#[test]
fn single_pattern_matching() {
    let filter: &str = "http/*";
    let globset =
        build_globset_internal(filter).expect("Failed to build glob set for a single pattern");
    assert!(globset.is_match("http/test1"));
    assert!(!globset.is_match("db/test1"));
}
#[test]
fn comma_separated_pattern_list_matching() {
    let filter: &str = "http/*,db/*";
    let globset = build_globset_internal(filter)
        .expect("Failed to build glob set for comma-separated patterns");
    assert!(globset.is_match("http/test1"));
    assert!(globset.is_match("db/test1"));
    assert!(!globset.is_match("cache/test1"));
}
#[test]
fn whitespace_handling_in_patterns() {
    let filter: &str = "  http/*  ,   db/*  ";
    let globset =
        build_globset_internal(filter).expect("Failed to build glob set with whitespace in filter");
    assert!(globset.is_match("http/test2"));
    assert!(globset.is_match("db/test2"));
    assert!(!globset.is_match("cache/test2"));
}
#[test]
fn empty_filter_handling() {
    let empty_filter: &str = "";
    let globset_empty =
        build_globset_internal(empty_filter).expect("Failed to build glob set for empty filter");
    assert!(!globset_empty.is_match("any/test"));
    let whitespace_filter: &str = "  ,   ,  ";
    let globset_whitespace = build_globset_internal(whitespace_filter)
        .expect("Failed to build glob set for whitespace-only filter");
    assert!(!globset_whitespace.is_match("any/test"));
}
#[test]
fn invalid_glob_pattern_returns_error() {
    let filter: &str = "[";
    let err = build_globset_internal(filter)
        .expect_err("Expected invalid glob pattern to return an error");
    assert!(err.contains("Invalid glob pattern '['"));
}
#[test]
fn list_flag() {
    let args = vec![
        "nanvix-test".to_string(),
        "-list".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse -list flag");
    assert!(parsed.list(), "-list flag must set list to true");
    assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
}
#[test]
fn list_flag_combined_with_test_filter() {
    let args = vec![
        "nanvix-test".to_string(),
        "-list".to_string(),
        "-test".to_string(),
        "http/*".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse -list with -test filter");
    assert!(parsed.list(), "-list flag must be set");
    assert_eq!(parsed.test_filter(), Some("http/*"));
    assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
}
#[test]
fn no_list_flag_defaults_to_false() {
    let args = vec![
        "nanvix-test".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse args without -list");
    assert!(!parsed.list(), "list must default to false");
}
#[test]
fn shard_flag_parses_valid_value() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "2/4".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse -shard flag");
    let shard = parsed.shard().expect("shard must be set");
    assert_eq!(shard.index(), 2, "one-based shard index must be 2");
    assert_eq!(shard.total(), 4, "shard total must be 4");
    assert_eq!(parsed.config_file_path(), "test/test-standalone.toml");
}
#[test]
fn shard_flag_defaults_to_none() {
    let args = vec![
        "nanvix-test".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse args without -shard");
    assert!(parsed.shard().is_none(), "shard must default to None");
}
#[test]
fn shard_selects_round_robin() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "1/4".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    let parsed = super::Args::parse(args).expect("Failed to parse -shard flag");
    let shard = parsed.shard().expect("shard must be set");
    // Shard 1 of 4 (zero-based index 0) selects positions 0, 4, 8, ...
    assert!(shard.selects(0));
    assert!(!shard.selects(1));
    assert!(!shard.selects(3));
    assert!(shard.selects(4));
}
#[test]
fn shard_flag_rejects_zero_total() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "1/0".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    assert!(super::Args::parse(args).is_err(), "zero TOTAL must be rejected");
}
#[test]
fn shard_flag_rejects_index_out_of_range() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "5/4".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    assert!(super::Args::parse(args).is_err(), "INDEX greater than TOTAL must be rejected");
}
#[test]
fn shard_flag_rejects_zero_index() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "0/4".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    assert!(super::Args::parse(args).is_err(), "zero INDEX must be rejected");
}
#[test]
fn shard_flag_rejects_malformed_value() {
    let args = vec![
        "nanvix-test".to_string(),
        "-shard".to_string(),
        "abc".to_string(),
        "test/test-standalone.toml".to_string(),
    ];
    assert!(super::Args::parse(args).is_err(), "malformed shard value must be rejected");
}
