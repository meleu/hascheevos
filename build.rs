//! Compiles the hashing part of the vendored rcheevos library (ADR-0001) and
//! generates Rust declarations for its current iterator API.

use std::env;
use std::path::{Path, PathBuf};

/// Compile-time switches shared by the C build and the generated declarations.
/// They change the layout of `rc_hash_iterator_t`, so both sides must agree.
///
/// 3DS encrypted hashing is compiled out. CHD needs no switch: rcheevos has no
/// CHD reader of its own, and we do not supply one.
const DEFINES: &[&str] = &["RC_HASH_NO_ENCRYPTED"];

/// The hashing sources, including the default disc-image reader (`cdreader.c`),
/// and the console names (`consoleinfo.c`).
const SOURCES: &[&str] = &[
    "src/rc_compat.c",
    "src/rcheevos/consoleinfo.c",
    "src/rhash/cdreader.c",
    "src/rhash/hash.c",
    "src/rhash/hash_disc.c",
    "src/rhash/hash_rom.c",
    "src/rhash/hash_zip.c",
    "src/rhash/md5.c",
];

/// Only the non-deprecated iterator API and the console-name function are
/// declared, so the deprecated global-callback functions cannot be called by
/// accident.
const FUNCTIONS: &[&str] = &[
    "rc_hash_initialize_iterator",
    "rc_hash_generate",
    "rc_hash_iterate",
    "rc_hash_destroy_iterator",
    "rc_console_name",
];

fn main() {
    let rcheevos = Path::new("vendor/rcheevos");
    let header = rcheevos.join("include/rc_hash.h");
    assert!(
        header.exists(),
        "{} not found: run `git submodule update --init --recursive`",
        header.display()
    );

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.git/modules/vendor/rcheevos/HEAD");

    let mut build = cc::Build::new();
    build.include(rcheevos.join("include")).warnings(false);
    for define in DEFINES {
        build.define(define, None);
    }
    for source in SOURCES {
        build.file(rcheevos.join(source));
    }
    build.compile("rcheevos_hash");

    let mut bindings = bindgen::Builder::default()
        .header(header.to_str().expect("header path is UTF-8"))
        .clang_arg(format!("-I{}", rcheevos.join("include").display()))
        .clang_args(DEFINES.iter().map(|define| format!("-D{define}")))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    for function in FUNCTIONS {
        bindings = bindings.allowlist_function(function);
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    bindings
        .generate()
        .expect("generate declarations from rc_hash.h")
        .write_to_file(out_dir.join("rcheevos.rs"))
        .expect("write generated declarations");
}
