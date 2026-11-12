// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::Args;

#[test]
fn parse_rejects_zero_payload_size() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-socket".to_string(),
        "-payload-size".to_string(),
        "0".to_string(),
    ];

    let error: anyhow::Error = match Args::parse(args) {
        Ok(_) => panic!("expected zero payload size to be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("positive integer"), "unexpected error: {error}");
}

#[test]
fn parse_accepts_warm_start_vmm_payload_size_override() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-vmm".to_string(),
        "-payload-size".to_string(),
        "4096".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected warm-start-vmm payload size to parse");

    assert_eq!(args.payload_size_override(), Some(4096));
}

#[test]
fn parse_accepts_cold_start_uvm() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "cold-start-uvm".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected cold-start-uvm to parse");

    assert!(matches!(args.benchmark(), super::BenchmarkFlavour::ColdStartUvm));
}

#[test]
fn parse_accepts_syscall_bench() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "syscall-bench".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected syscall-bench to parse");

    assert!(matches!(args.benchmark(), super::BenchmarkFlavour::SyscallBench));
}

#[test]
fn parse_accepts_warm_start_gateway_payload_size_override() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-gateway".to_string(),
        "-payload-size".to_string(),
        "4096".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected warm-start-gateway payload size to parse");

    assert_eq!(args.payload_size_override(), Some(4096));
}

#[test]
fn parse_accepts_warm_start_socket_payload_size_override() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-socket".to_string(),
        "-payload-size".to_string(),
        "4096".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected warm-start-socket payload size to parse");

    assert_eq!(args.payload_size_override(), Some(4096));
}

#[test]
fn parse_keeps_warm_start_socket_payload_size_override_empty_when_omitted() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-socket".to_string(),
    ];

    let args: Args = Args::parse(args).expect("expected warm-start-socket to parse");

    assert_eq!(args.payload_size_override(), None);
}

#[test]
fn parse_rejects_too_small_warm_start_vmm_payload_size() {
    let args: Vec<String> = vec![
        "nanvix-bench".to_string(),
        "-benchmark".to_string(),
        "warm-start-vmm".to_string(),
        "-payload-size".to_string(),
        "2".to_string(),
    ];

    let error: anyhow::Error = match Args::parse(args) {
        Ok(_) => panic!("expected too-small warm-start-vmm payload size to be rejected"),
        Err(error) => error,
    };

    assert!(error.to_string().contains(">= 4"), "unexpected error: {error}");
}
