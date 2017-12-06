use std::process::Command;
use std::path::Path;

fn main() {
    let out = std::env::var("OUT_DIR").unwrap();

    Command::new("make")
        .arg("NO_PROTOBUF=1")
        .arg("bin/libosm.a")
        .current_dir("lib/openstreetmap-fun")
        .status()
        .unwrap();

    Command::new("mv")
        .arg("lib/openstreetmap-fun/bin/libosm.a")
        .arg(&out)
        .status()
        .unwrap();

    println!("cargo:rustc-link-lib=static=osm");
    println!("cargo:rustc-link-search=native={}", out);
}