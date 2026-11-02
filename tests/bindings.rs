use std::{env, fs};

const COMMITTED: &str = "src/rc_hash/bindings.rs";

// Run with UPDATE_BINDINGS=1 after bumping the rcheevos submodule.
#[test]
fn committed_bindings_match_the_rcheevos_headers() {
    let generated = bindgen::Builder::default()
        .header("vendor/rcheevos/include/rc_hash.h")
        // Keep in sync with the defines in build.rs.
        .clang_args([
            "-Ivendor/rcheevos/include",
            "-D_LARGEFILE64_SOURCE",
            "-D_FILE_OFFSET_BITS=64",
        ])
        .allowlist_function(
            "rc_hash_(initialize_iterator|destroy_iterator|iterate|generate|get_default_cdreader)",
        )
        .allowlist_var("RC_CONSOLE_.*")
        .disable_header_comment()
        .layout_tests(false)
        .generate()
        .expect("generate bindings")
        .to_string();

    if env::var_os("UPDATE_BINDINGS").is_some() {
        fs::write(COMMITTED, &generated).expect("write bindings");
    }

    let committed = fs::read_to_string(COMMITTED).unwrap_or_default();
    assert!(
        committed == generated,
        "{COMMITTED} is stale; run `UPDATE_BINDINGS=1 cargo test --test bindings`"
    );
}
