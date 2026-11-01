//! Bumps a build counter (build_number.txt) whenever the sources change and
//! bakes the number and build time into the binary.
use std::{fs, time::{SystemTime, UNIX_EPOCH}};

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");

    let n = fs::read_to_string("build_number.txt")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
        + 1;
    let _ = fs::write("build_number.txt", n.to_string());

    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=BUILD_NUMBER={n}");
    println!("cargo:rustc-env=BUILD_UNIX={t}");
}
