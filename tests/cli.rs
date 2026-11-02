//! Expected hashes come from RAHasher 1.8.4, unless noted otherwise.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SNES_SFC: &str = "6556112372898c69e1de0bf689d8db26";
const PSX: &str = "090500155aa193ff78586fa532a36023";
const SEGA_CD: &str = "206d293cdd4e514fffdd4057d7f7c9fa";
const GENERIC_ROM: &str = "6ed56530d641cd3cc1c9921fe3327b4a";
const BOTH_PSX: &str = "090500155aa193ff78586fa532a36023";
const BOTH_SEGA_CD: &str = "206d293cdd4e514fffdd4057d7f7c9fa";

fn hash(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hascheevos"))
        .arg("hash")
        .args(args)
        .output()
        .expect("run hascheevos")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[track_caller]
fn assert_hashes(args: &[&str], expected: &[&str]) {
    let output = hash(args);
    let lines: Vec<String> = stdout(&output).lines().map(str::to_owned).collect();
    assert_eq!(lines, expected, "stderr: {}", stderr(&output));
    assert!(output.status.success());
}

#[track_caller]
fn assert_fails(args: &[&str], message: &str) {
    let output = hash(args);
    assert_eq!(stdout(&output), "");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains(message),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn hashes_a_rom_for_a_named_console() {
    assert_hashes(&["--console", "snes", "testdata/snes/FakeHeaderedROM.sfc"], &[SNES_SFC]);
}

#[test]
fn accepts_a_numeric_console_id() {
    assert_hashes(&["--console", "3", "testdata/snes/FakeHeaderedROM.sfc"], &[SNES_SFC]);
}

#[test]
fn finds_the_console_from_the_extension() {
    assert_hashes(&["testdata/snes/FakeHeaderedROM.sfc"], &[SNES_SFC]);
}

#[test]
fn hashes_disc_formats() {
    assert_hashes(&["--console", "psx", "testdata/disc/psx.cue"], &[PSX]);
    assert_hashes(&["--console", "psx", "testdata/disc/psx.bin"], &[PSX]);
    assert_hashes(&["--console", "psx", "testdata/disc/psx.m3u"], &[PSX]);
    assert_hashes(&["--console", "segacd", "testdata/disc/segacd.iso"], &[SEGA_CD]);
    assert_hashes(&["--console", "segacd", "testdata/disc/segacd.gdi"], &[SEGA_CD]);
}

#[test]
fn lists_every_distinct_candidate_hash_without_a_console() {
    // RAHasher with `12 both.cue` and `9 both.cue`.
    assert_hashes(&["testdata/disc/both.cue"], &[BOTH_PSX, BOTH_SEGA_CD]);
}

#[test]
fn repeated_candidate_hashes_are_printed_once() {
    // Every console tried for a .rom file hashes the whole file.
    assert_hashes(&["testdata/generic-file.rom"], &[GENERIC_ROM]);
}

#[test]
fn candidate_failures_stay_quiet_when_a_hash_is_found() {
    let output = hash(&["testdata/disc/psx.cue"]);
    assert!(output.status.success());
    assert_eq!(stderr(&output), "");
}

#[test]
fn verbose_reports_progress_on_stderr() {
    let output = hash(&["-v", "--console", "snes", "testdata/snes/FakeHeaderedROM.sfc"]);
    assert_eq!(stdout(&output).trim(), SNES_SFC);
    assert!(stderr(&output).contains("Generated hash"), "stderr: {}", stderr(&output));
}

#[test]
fn unknown_extension_prints_the_whole_file_hash_and_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.unknownext");
    fs::copy("testdata/generic-file.rom", &path).unwrap();

    let output = hash(&[path.to_str().unwrap()]);
    assert_eq!(stdout(&output).trim(), GENERIC_ROM); // same bytes as `md5sum`
    assert!(output.status.success());
    assert!(stderr(&output).contains("warning"), "stderr: {}", stderr(&output));
}

#[test]
fn rcheevos_errors_are_reported_when_no_hash_is_produced() {
    assert_fails(
        &["--console", "3ds", "testdata/disc/encrypted.3ds"],
        "normal keys callback was not set",
    );
}

#[test]
fn missing_files_fail() {
    assert_fails(&["testdata/does-not-exist.sfc"], "testdata/does-not-exist.sfc");
}

#[test]
fn directories_only_work_for_arcade() {
    assert!(Path::new("testdata/snes").is_dir());
    assert_fails(&["--console", "snes", "testdata/snes"], "not a file");
}

#[test]
fn unknown_consoles_point_to_the_help_list() {
    assert_fails(&["--console", "notaconsole", "testdata/generic-file.rom"], "--help");
    assert_fails(&["--console", "100", "testdata/generic-file.rom"], "--help");
}
