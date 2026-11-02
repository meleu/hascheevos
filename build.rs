use std::path::Path;

const RCHEEVOS: &str = "vendor/rcheevos";
const SOURCES: &[&str] = &[
    "src/rhash/aes.c",
    "src/rhash/cdreader.c",
    "src/rhash/hash.c",
    "src/rhash/hash_disc.c",
    "src/rhash/hash_encrypted.c",
    "src/rhash/hash_rom.c",
    "src/rhash/hash_zip.c",
    "src/rhash/md5.c",
];

fn main() {
    let root = Path::new(RCHEEVOS);
    assert!(
        root.join("include/rc_hash.h").exists(),
        "{RCHEEVOS} is empty; run `git submodule update --init`"
    );

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={RCHEEVOS}/include");
    println!("cargo:rerun-if-changed={RCHEEVOS}/src/rhash");

    // Don't set a strict C standard: these sources need the GNU extensions
    // (strcasecmp, strdup) that `-std=c99` hides.
    let mut build = cc::Build::new();
    build
        .include(root.join("include"))
        .define("_LARGEFILE64_SOURCE", None)
        .define("_FILE_OFFSET_BITS", "64")
        .warnings(false);
    for source in SOURCES {
        build.file(root.join(source));
    }
    build.compile("rcheevos");
}
