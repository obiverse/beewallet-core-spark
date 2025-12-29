//! Build script for beewallet-core-spark
//!
//! Compiles the mobi C library from the vendor/mobi submodule.

fn main() {
    // Compile mobi C library
    cc::Build::new()
        .file("vendor/mobi/src/mobi.c")
        .include("vendor/mobi/src")
        .flag("-std=c99")
        .flag("-O2")
        .warnings(true)
        .compile("mobi");

    // Tell cargo to re-run if mobi source changes
    println!("cargo:rerun-if-changed=vendor/mobi/src/mobi.c");
    println!("cargo:rerun-if-changed=vendor/mobi/src/mobi.h");
}
