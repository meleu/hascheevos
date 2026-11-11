//! Integration tests for `hc hash`, run against the built binary.

use assert_cmd::Command;
use predicates::prelude::*;

const SNES: &str = "3";
const MEGA_DRIVE: &str = "1";
const NINTENDO_DS: &str = "18";

fn hc() -> Command {
    Command::cargo_bin("hc").expect("the hc binary is built")
}

fn fixture(relative: &str) -> String {
    format!("{}/tests/fixtures/{relative}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn headered_snes_rom_hashes_without_its_header() {
    // MD5 of the fixture with its first 512 bytes removed.
    hc().args([
        "hash",
        "--console",
        SNES,
        &fixture("snes/FakeHeaderedROM.sfc"),
    ])
    .assert()
    .success()
    .stdout("6556112372898c69e1de0bf689d8db26\n");
}

#[test]
fn generic_file_hashes_as_its_whole_content() {
    // MD5 of the whole fixture: Mega Drive's official method.
    hc().args([
        "hash",
        "--console",
        MEGA_DRIVE,
        &fixture("generic-file.rom"),
    ])
    .assert()
    .success()
    .stdout("6ed56530d641cd3cc1c9921fe3327b4a\n");
}

#[test]
fn missing_rom_fails_with_empty_stdout() {
    hc().args(["hash", "--console", SNES, &fixture("no-such.sfc")])
        .assert()
        .failure()
        .stdout("")
        .stderr(
            predicate::str::contains("cannot read '")
                .and(predicate::str::contains("no-such.sfc':")),
        );
}

#[test]
fn rom_the_console_cannot_hash_fails_with_empty_stdout() {
    // 96 bytes is too short to hold a Nintendo DS header.
    hc().args([
        "hash",
        "--console",
        NINTENDO_DS,
        &fixture("generic-file.rom"),
    ])
    .assert()
    .failure()
    .stdout("")
    .stderr(predicate::str::contains("produced no hash for '"));
}

#[test]
fn non_numeric_console_is_rejected() {
    hc().args(["hash", "--console", "snes", &fixture("generic-file.rom")])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--console"));
}

#[test]
fn more_than_one_rom_is_rejected() {
    let rom = fixture("generic-file.rom");
    hc().args(["hash", "--console", MEGA_DRIVE, &rom, &rom])
        .assert()
        .failure()
        .stdout("");
}

#[test]
fn no_arguments_explains_usage() {
    hc().arg("hash")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Usage: hc hash"));
}

#[test]
fn help_explains_usage() {
    hc().args(["hash", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: hc hash"));
}

#[test]
fn top_level_help_describes_the_tool() {
    hc().arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Tells whether a ROM is supported by RetroAchievements",
        ));
}

#[test]
fn directory_given_as_rom_fails_with_empty_stdout() {
    hc().args(["hash", "--console", SNES, &fixture("snes")])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("snes' is not a file"));
}

/// Splits `hc hash` output without `--console` into its lines, checking each
/// is a 32-hex-character hash, a numeric console ID and a console name.
fn console_hash_lines(stdout: &[u8]) -> Vec<(String, String, String)> {
    let stdout = std::str::from_utf8(stdout).expect("stdout is UTF-8");
    assert!(
        stdout.ends_with('\n'),
        "stdout ends with a newline: {stdout:?}"
    );
    stdout
        .lines()
        .map(|line| {
            let columns: Vec<&str> = line.split('\t').collect();
            let [hash, console, name] = columns[..] else {
                panic!("line has three tab-separated columns: {line:?}");
            };
            assert!(
                hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                "first column is a 32-hex-character hash: {line:?}"
            );
            assert!(
                console.parse::<u32>().is_ok(),
                "second column is a numeric console ID: {line:?}"
            );
            assert!(!name.is_empty(), "third column is a console name: {line:?}");
            (hash.to_owned(), console.to_owned(), name.to_owned())
        })
        .collect()
}

#[test]
fn unknown_console_lists_console_hashes_including_the_expected_console() {
    let output = hc()
        .args(["hash", &fixture("snes/FakeHeaderedROM.sfc")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let lines = console_hash_lines(&output);
    assert!(
        lines.contains(&(
            "6556112372898c69e1de0bf689d8db26".to_owned(),
            SNES.to_owned(),
            "Super Nintendo Entertainment System".to_owned()
        )),
        "the SNES console hash is listed: {lines:?}"
    );
}

/// Copies the generic-file fixture to `name` in a scratch directory, so its
/// extension steers rcheevos' choice of plausible consoles.
fn generic_file_named(name: &str) -> String {
    let copy = format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"));
    std::fs::copy(fixture("generic-file.rom"), &copy).expect("copy the generic-file fixture");
    copy
}

#[test]
fn unknown_console_lists_the_same_hash_once_per_console() {
    // rcheevos tries Apple II, then Commodore 64, for a `.nib` file; both
    // hash the whole file.
    hc().args(["hash", &generic_file_named("same-hash.nib")])
        .assert()
        .success()
        .stdout(
            "6ed56530d641cd3cc1c9921fe3327b4a\t38\tApple II\n\
             6ed56530d641cd3cc1c9921fe3327b4a\t30\tCommodore 64\n",
        );
}

#[test]
fn printed_console_id_gives_the_hash_from_its_line() {
    for rom in [
        fixture("snes/FakeHeaderedROM.sfc"),
        generic_file_named("round-trip.nib"),
    ] {
        let output = hc()
            .args(["hash", &rom])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        for (hash, console, _) in console_hash_lines(&output) {
            hc().args(["hash", "--console", &console, &rom])
                .assert()
                .success()
                .stdout(format!("{hash}\n"));
        }
    }
}

#[test]
fn rom_no_console_can_hash_fails_with_empty_stdout() {
    // Only disc consoles are tried for a `.iso` file, and 96 bytes is no disc.
    hc().args(["hash", &generic_file_named("no-console.iso")])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("no console produced a hash for '"));
}
