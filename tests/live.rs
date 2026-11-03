//! Confirms against the real RetroAchievements API what the fake server can
//! only assume. It needs a key, so plain `cargo test` skips it:
//!
//! ```text
//! RA_API_KEY=... cargo test --test live -- --ignored --nocapture
//! ```

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;
use std::process::{Command, Output};

/// A game only one console has, from each of a few consoles small enough to
/// download together within the server's limit of 60 requests a minute.
const KNOWN_GAMES: [(&str, &str, &str); 4] = [
    (
        "nes",
        "0e2be2324567ecbc373d9ef562dac8b9",
        "Super Mario Bros.",
    ),
    (
        "megadrive",
        "1b1d9ac862c387367e904036114c4825",
        "Sonic the Hedgehog",
    ),
    ("pcengine", "0e0040142b5227f2b85be27364d5ea01", "R-Type"),
    // The server has this one in uppercase.
    (
        "virtualboy",
        "b1b3a25ce8fb4f406bf82b8765d307d0",
        "Mario Clash",
    ),
];

/// From the same lists as [`KNOWN_GAMES`].
const SUPER_MARIO_BROS: (&str, &str) = ("1446", "Super Mario Bros");

/// Far beyond any game the server has.
const NO_SUCH_GAME: &str = "999999999";

const COMPATIBILITY_VALUES: [&str; 4] =
    ["compatible", "incompatible", "patch-required", "untested"];

/// 346 games in 2026, at most 100 to a page.
const PC_ENGINE: &str = "8";

fn hascheevos(cache: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hascheevos"));
    command
        .args(args)
        .env_remove("RA_API_URL")
        .env("XDG_CACHE_HOME", cache);
    command
}

fn run(mut command: Command) -> Output {
    let output = command.output().expect("run hascheevos");
    // Shows rate limiting and progress when run with `--nocapture`.
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    output
}

#[test]
#[ignore = "needs RA_API_KEY and the real API"]
fn the_real_api_behaves_as_the_fake_server_assumes() {
    let key = std::env::var("RA_API_KEY").unwrap_or_default();
    assert!(!key.is_empty(), "this test needs RA_API_KEY");
    let cache = tempfile::tempdir().expect("create cache directory");

    for (console, hash, title) in KNOWN_GAMES {
        let output = run(hascheevos(
            cache.path(),
            &["md5", "--console", console, hash],
        ));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(0), "{console}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), hash);
        assert!(stderr.contains(title), "{console}: {stderr}");
    }

    let (id, title) = SUPER_MARIO_BROS;
    let output = run(hascheevos(cache.path(), &["game", id]));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), id);
    assert!(stderr.contains(title), "{stderr}");

    let output = run(hascheevos(cache.path(), &["game", NO_SUCH_GAME]));
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");

    // More games than one page holds shows that every page was downloaded, and
    // an unexpected value here would exclude its ROMs, so it needs a look.
    let output = run(hascheevos(
        cache.path(),
        &["__fetch-system", "--format", "0", PC_ENGINE],
    ));
    assert_eq!(output.status.code(), Some(0));
    let list: Map<String, Value> = serde_json::from_slice(&output.stdout).expect("list as JSON");
    let games: HashSet<&str> = list
        .values()
        .filter_map(|game| game["id"].as_str())
        .collect();
    assert!(games.len() > 100, "{} games", games.len());
    for (hash, game) in &list {
        let compatibility = game["compatibility"].as_str().unwrap_or_default();
        assert!(
            COMPATIBILITY_VALUES.contains(&compatibility),
            "{hash}: {compatibility}"
        );
    }

    let mut rejected = hascheevos(
        cache.path(),
        &["__fetch-system", "--format", "0", PC_ENGINE],
    );
    rejected.env("RA_API_KEY", "not-a-real-key");
    assert_eq!(run(rejected).status.code(), Some(3));
}
