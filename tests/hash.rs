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
        .stderr(predicate::str::contains("no-such.sfc"));
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
    .stderr(predicate::str::contains("produced no hash"));
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
fn directory_given_as_rom_fails_with_empty_stdout() {
    hc().args(["hash", "--console", SNES, &fixture("snes")])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("not a file"));
}
