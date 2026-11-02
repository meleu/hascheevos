//! Expected hashes come from RAHasher 1.8.4, unless noted otherwise.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

const SNES_SFC: &str = "6556112372898c69e1de0bf689d8db26";
const PSX: &str = "090500155aa193ff78586fa532a36023";
const SEGA_CD: &str = "206d293cdd4e514fffdd4057d7f7c9fa";
const GENERIC_ROM: &str = "6ed56530d641cd3cc1c9921fe3327b4a";
const CHRISTMAS_CRAZE: &str = "0829ebfde80e7a9673fe9966a8d47b0f";
const CHRISTMAS_CRAZE_ARCADE: &str = "b7d59e4f4dff40e10f716433ccec74fa";
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
    assert_hashes(
        &["--console", "snes", "testdata/snes/FakeHeaderedROM.sfc"],
        &[SNES_SFC],
    );
}

#[test]
fn accepts_a_numeric_console_id() {
    assert_hashes(
        &["--console", "3", "testdata/snes/FakeHeaderedROM.sfc"],
        &[SNES_SFC],
    );
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
    assert_hashes(
        &["--console", "segacd", "testdata/disc/segacd.iso"],
        &[SEGA_CD],
    );
    assert_hashes(
        &["--console", "segacd", "testdata/disc/segacd.gdi"],
        &[SEGA_CD],
    );
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
    let output = hash(&[
        "-v",
        "--console",
        "snes",
        "testdata/snes/FakeHeaderedROM.sfc",
    ]);
    assert_eq!(stdout(&output).trim(), SNES_SFC);
    assert!(
        stderr(&output).contains("Generated hash"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn unknown_extension_prints_the_whole_file_hash_and_a_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.unknownext");
    fs::copy("testdata/generic-file.rom", &path).unwrap();

    let output = hash(&[path.to_str().unwrap()]);
    assert_eq!(stdout(&output).trim(), GENERIC_ROM); // same bytes as `md5sum`
    assert!(output.status.success());
    assert!(
        stderr(&output).contains("warning"),
        "stderr: {}",
        stderr(&output)
    );
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
    assert_fails(
        &["testdata/does-not-exist.sfc"],
        "testdata/does-not-exist.sfc",
    );
}

#[test]
fn directories_only_work_for_arcade() {
    assert!(Path::new("testdata/snes").is_dir());
    assert_fails(&["--console", "snes", "testdata/snes"], "not a file");
}

#[test]
fn unknown_consoles_point_to_the_help_list() {
    assert_fails(
        &["--console", "notaconsole", "testdata/generic-file.rom"],
        "--help",
    );
    assert_fails(&["--console", "100", "testdata/generic-file.rom"], "--help");
}

#[test]
fn hashes_the_first_entry_of_a_zip() {
    assert_hashes(
        &["--console", "nes", "testdata/nes/Zooming-Secretary.zip"],
        &["bed0f7b12673dd762eed665c5c61927b"],
    );
    assert_hashes(
        &[
            "--console",
            "megadrive",
            "testdata/megadrive/Abbaye-des-Morts.zip",
        ],
        &["bc2e5590cc0e7b6e863b4275343839d4"],
    );
    assert_hashes(
        &["--console", "n64", "testdata/n64/pyoro64-ntsc.zip"],
        &["2ac0f88bea02d67896ffeae9e8ec72e7"],
    );
    assert_hashes(
        &[
            "--console",
            "atari2600",
            "testdata/atari2600/Wall-Jump-Ninja.zip",
        ],
        &["3c56c0c5f6f97850ed0aa7bcc2a4e30e"],
    );
}

#[test]
fn zip_without_a_console_uses_the_extension_of_its_first_entry() {
    assert_hashes(&["testdata/snes/Christmas-Craze.zip"], &[CHRISTMAS_CRAZE]);
}

#[test]
fn arcade_hashes_the_archive_name_without_extracting() {
    assert_hashes(
        &["--console", "arcade", "testdata/snes/Christmas-Craze.zip"],
        &[CHRISTMAS_CRAZE_ARCADE],
    );
    assert_hashes(
        &["--console", "fbneo", "testdata/snes/Christmas-Craze.zip"],
        &[CHRISTMAS_CRAZE_ARCADE],
    );
}

#[test]
fn seven_zip_is_only_accepted_for_arcade() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Christmas-Craze.7z");
    fs::write(&path, b"not read").unwrap();
    let path = path.to_str().unwrap();

    assert_fails(&[path], "not supported yet");
    assert_fails(&["--console", "snes", path], "not supported yet");
    assert_hashes(&["--console", "arcade", path], &[CHRISTMAS_CRAZE_ARCADE]);
}

#[test]
fn chd_is_not_supported_yet() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.CHD");
    fs::write(&path, b"not read").unwrap();
    let path = path.to_str().unwrap();

    assert_fails(&[path], "not supported yet");
    assert_fails(&["--console", "psx", path], "not supported yet");
    assert_fails(&["--console", "arcade", path], "not supported yet");
}

#[test]
fn nested_archives_never_fall_through_to_a_filename_hash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("outer.zip");
    let mut archive = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    archive
        .start_file("inner.zip", zip::write::SimpleFileOptions::default())
        .unwrap();
    archive.write_all(b"not read").unwrap();
    archive.finish().unwrap();

    assert_fails(&[path.to_str().unwrap()], "not supported yet");
}

#[test]
fn zips_with_no_files_fail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.zip");
    zip::ZipWriter::new(fs::File::create(&path).unwrap())
        .finish()
        .unwrap();

    assert_fails(&[path.to_str().unwrap()], "no files");
}

#[test]
fn help_lists_console_names_with_aliases() {
    let output = hash(&["--help"]);
    let help = stdout(&output);
    for name in [
        "megadrive (genesis)",
        "snes",
        "gba",
        "3ds",
        "arcade (fbneo, fba)",
    ] {
        assert!(help.contains(name), "{help}");
    }
}

#[test]
fn playlists_of_unsupported_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    for (name, first_disc) in [
        ("chd.m3u", "game.chd"),
        ("zip.m3u", "# comment\n\ngame.zip"),
    ] {
        let path = dir.path().join(name);
        fs::write(&path, first_disc).unwrap();
        assert_fails(&[path.to_str().unwrap()], "not supported yet");
    }
}
