// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

pub fn get_proj_root() -> String {
    format!("{}/../../..", env!("CARGO_MANIFEST_DIR"))
}

