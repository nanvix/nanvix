// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// Tests that `NetBackend::default()` creates a valid instance.
#[test]
#[allow(clippy::default_constructed_unit_structs)]
fn default_creates_backend() {
    let _backend: NetBackend = NetBackend::default();
}

/// Tests that `NetBackend::new()` creates a valid instance.
#[test]
fn new_creates_backend() {
    let _backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
}
