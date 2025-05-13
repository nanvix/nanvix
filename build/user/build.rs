// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![deny(clippy::all)]

//==================================================================================================
// Imports
//==================================================================================================

use std::{
    env,
    fs::File,
    io::Read,
    path::{
        Path,
        PathBuf,
    },
};

//==================================================================================================
// Main Function
//==================================================================================================

fn main() {
    //==============================================================================================
    //Benchmark-Specific Environment Variables
    //==============================================================================================

    // Get CARGO_MANIFEST_DIR Environment Variable.
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(mdir) => mdir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };

    let config_path: PathBuf = Path::new(&manifest_dir).join("config.json");
    if config_path.exists() {
        println!("cargo:rerun-if-changed=./config.json");
        let mut file: File = File::open(&config_path).expect("Failed to open file");
        let mut raw_content: String = String::new();
        let _ = file.read_to_string(&mut raw_content);
        let content = raw_content.replace("\n", "").replace(" ", "");
        println!("cargo:rustc-env=CONFIG={}", content);
    }

    //==============================================================================================
    // Link Archive
    //==============================================================================================

    println!("cargo::rustc-link-arg=-Tbuild/user/linker/x86/user.ld");

    //==============================================================================================

    // Get CARGO_MANIFEST_DIR Environment Variable.
    let manifest_dir: String = match env::var("CARGO_MANIFEST_DIR") {
        Ok(mdir) => mdir,
        Err(_) => panic!("failed to get CARGO_MANIFEST_DIR environment variable"),
    };

    let config_path: PathBuf = Path::new(&manifest_dir).join("config.json");
    if config_path.exists() {
        println!("cargo:rerun-if-changed=config.json");
        let mut file: File = File::open(&config_path).expect("Failed to open config.json file");
        let mut raw_content: String = String::new();
        let _ = file.read_to_string(&mut raw_content);
        let content = raw_content.replace("\n", "").replace(" ", "");
        println!("cargo:rustc-env=CONFIG={}", content);
    }
}
