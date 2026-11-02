// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

fn base_args() -> Vec<String> {
    vec![
        "uservm".to_string(),
        Args::OPT_KERNEL.to_string(),
        "kernel.elf".to_string(),
    ]
}

#[test]
fn parse_defaults_user_vm_identifier() {
    let args: Args = Args::parse(base_args()).expect("arguments should parse");
    assert_eq!(u32::from(args.user_vm_id()), 0);
    assert_eq!(args.kernel_filename(), "kernel.elf");
}

#[test]
fn parse_accepts_user_vm_identifier_and_images() {
    let mut raw_args: Vec<String> = base_args();
    raw_args.extend([
        Args::OPT_USER_VM_ID.to_string(),
        "7".to_string(),
        Args::OPT_INITRD.to_string(),
        "initrd.img".to_string(),
        Args::OPT_RAMFS.to_string(),
        "ramfs.img".to_string(),
    ]);
    let mut args: Args = Args::parse(raw_args).expect("arguments should parse");
    assert_eq!(u32::from(args.user_vm_id()), 7);
    assert_eq!(args.initrd_filename().as_deref(), Some("initrd.img"));
    assert_eq!(args.ramfs_filename().as_deref(), Some("ramfs.img"));
}

#[test]
fn parse_rejects_missing_kernel() {
    let result: Result<Args> = Args::parse(vec!["uservm".to_string()]);
    assert!(result.is_err());
}

#[cfg(feature = "gdb")]
#[test]
fn parse_accepts_gdb_port() {
    let mut raw_args: Vec<String> = base_args();
    raw_args.extend([Args::OPT_GDB_PORT.to_string(), "1234".to_string()]);
    let args: Args = Args::parse(raw_args).expect("GDB port should parse");
    assert_eq!(args.gdb_port(), Some(1234));
}

#[cfg(feature = "gdb")]
#[test]
fn parse_rejects_invalid_gdb_port() {
    let mut raw_args: Vec<String> = base_args();
    raw_args.extend([Args::OPT_GDB_PORT.to_string(), "invalid".to_string()]);
    assert!(Args::parse(raw_args).is_err());
}
