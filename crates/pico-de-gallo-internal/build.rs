use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let major = env!("CARGO_PKG_VERSION_MAJOR")
        .parse::<u16>()
        .expect("should have major version");

    let minor = env!("CARGO_PKG_VERSION_MINOR")
        .parse::<u16>()
        .expect("should have minor version");

    let patch = env!("CARGO_PKG_VERSION_PATCH")
        .parse::<u32>()
        .expect("should have patch-level version");

    File::create(out.join("schema_version.rs"))
        .unwrap()
        .write_all(
            format!(
                r##"
/// Schema version major — derived from the pico-de-gallo-internal crate version.
pub const SCHEMA_VERSION_MAJOR: u16 = {major};

/// Schema version minor — derived from the pico-de-gallo-internal crate version.
pub const SCHEMA_VERSION_MINOR: u16 = {minor};

/// Schema version patch — derived from the pico-de-gallo-internal crate version.
pub const SCHEMA_VERSION_PATCH: u32 = {patch};
"##
            )
            .as_bytes(),
        )
        .unwrap();
}
