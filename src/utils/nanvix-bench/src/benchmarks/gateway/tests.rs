// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn gateway_payloads_use_legacy_sweep() {
    let payloads: Vec<(String, Vec<u8>)> =
        gateway_payloads(None).expect("default gateway payloads should be valid");
    let labels: Vec<&str> = payloads.iter().map(|(label, _)| label.as_str()).collect();

    assert_eq!(labels, ["32B", "64B", "128B", "256B", "512B", "1KiB", "4KiB"]);
}

#[test]
fn gateway_payloads_honor_size_override() {
    let payloads: Vec<(String, Vec<u8>)> =
        gateway_payloads(Some(1024)).expect("gateway payload override should be valid");

    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0].0, "1KiB");
    assert_eq!(payloads[0].1.len(), 1024);
}

#[test]
fn gateway_payloads_reject_oversized_request() {
    let error: anyhow::Error = gateway_payloads(Some(MAX_PAYLOAD_SIZE + 1))
        .expect_err("oversized gateway payload should fail");

    assert!(error.to_string().contains("at most"), "unexpected error: {error}");
}
