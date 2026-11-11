//! Integration tests for `hc hash`, run against the built binary.

use assert_cmd::Command;
use predicates::prelude::*;

const SNES: &str = "3";
const MEGA_DRIVE: &str = "1";
const NINTENDO_DS: &str = "18";
const NINTENDO_64: &str = "2";
const NES: &str = "7";
const ATARI_2600: &str = "25";
const ARCADE: &str = "27";

/// The NES fixture's hash, as computed by the `go` branch.
const ZOOMING_SECRETARY: &str = "bed0f7b12673dd762eed665c5c61927b";

fn hc() -> Command {
    Command::cargo_bin("hc").expect("the hc binary is built")
}

fn fixture(relative: &str) -> String {
    format!("{}/tests/fixtures/{relative}", env!("CARGO_MANIFEST_DIR"))
}

/// A path named `name` in a scratch directory for files made at test time.
fn scratch(name: &str) -> String {
    format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"))
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
    let copy = scratch(name);
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

#[test]
fn help_explains_usage_and_the_columns_printed_without_a_console() {
    hc().args(["hash", "--help"]).assert().success().stdout(
        predicate::str::contains("Usage: hc hash").and(predicate::str::contains(
            "<hash><TAB><console id><TAB><console name>",
        )),
    );
}

#[test]
fn piped_stdout_gets_no_column_header() {
    // The tests capture stdout, so it is never a terminal here.
    hc().args(["hash", &fixture("snes/FakeHeaderedROM.sfc")])
        .assert()
        .success()
        .stderr(predicate::str::contains("CONSOLE ID").not());
}

#[test]
fn zipped_rom_hashes_as_the_rom_inside() {
    // MD5 over the bytes each console's official method covers, computed
    // outside rcheevos: the whole ROM, except NES without its 16-byte iNES
    // header and N64 in big-endian (.z64) byte order.
    for (console, rom, hash) in [
        (
            ATARI_2600,
            "atari2600/Wall-Jump-Ninja.zip",
            "3c56c0c5f6f97850ed0aa7bcc2a4e30e",
        ),
        (
            MEGA_DRIVE,
            "megadrive/Abbaye-des-Morts.zip",
            "bc2e5590cc0e7b6e863b4275343839d4",
        ),
        (
            NINTENDO_64,
            "n64/pyoro64-ntsc.zip",
            "2ac0f88bea02d67896ffeae9e8ec72e7",
        ),
        (NES, "nes/Zooming-Secretary.zip", ZOOMING_SECRETARY),
        (
            SNES,
            "snes/Christmas-Craze.zip",
            "0829ebfde80e7a9673fe9966a8d47b0f",
        ),
    ] {
        hc().args(["hash", "--console", console, &fixture(rom)])
            .assert()
            .success()
            .stdout(format!("{hash}\n"));
    }
}

/// Extracts the first entry of a fixture zip into a scratch directory,
/// returning the path of the extracted ROM.
fn unzipped(rom: &str) -> String {
    let file = std::fs::File::open(fixture(rom)).expect("open the fixture zip");
    let mut archive = zip::ZipArchive::new(file).expect("read the fixture zip");
    let mut entry = archive.by_index(0).expect("the fixture zip has an entry");
    let name = entry.name().expect("the entry has a name").into_owned();
    let path = scratch(&name);
    let mut out = std::fs::File::create(&path).expect("create the extracted ROM");
    std::io::copy(&mut entry, &mut out).expect("extract the ROM");
    path
}

fn hash_with_console(console: &str, rom: &str) -> Vec<u8> {
    hc().args(["hash", "--console", console, rom])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone()
}

#[test]
fn zipped_rom_hashes_the_same_as_the_rom_uncompressed() {
    for (console, rom) in [
        (NES, "nes/Zooming-Secretary.zip"),
        (NINTENDO_64, "n64/pyoro64-ntsc.zip"),
    ] {
        assert_eq!(
            hash_with_console(console, &fixture(rom)),
            hash_with_console(console, &unzipped(rom)),
            "{rom}"
        );
    }
}

#[test]
fn arcade_zip_hashes_as_its_name_without_extension() {
    // Arcade's official method reads the zip's name, never its content, so an
    // empty file proves the zip reaches rcheevos without being unpacked.
    let rom = scratch("mslug.zip");
    std::fs::write(&rom, b"").expect("create an empty arcade zip");
    hc().args(["hash", "--console", ARCADE, &rom])
        .assert()
        .success()
        // MD5 of "mslug".
        .stdout("b43c8b4ec999588c04dad79bb8bcc745\n");
}

#[test]
fn zip_extension_matches_in_any_letter_case() {
    let rom = scratch("Zooming-Secretary.ZIP");
    std::fs::copy(fixture("nes/Zooming-Secretary.zip"), &rom).expect("copy the NES fixture");
    hc().args(["hash", "--console", NES, &rom])
        .assert()
        .success()
        .stdout(format!("{ZOOMING_SECRETARY}\n"));
}

#[test]
fn empty_or_unreadable_zip_fails_with_empty_stdout() {
    // The smallest valid zip: an end-of-central-directory record, no entries.
    let mut empty_zip = b"PK\x05\x06".to_vec();
    empty_zip.resize(22, 0);
    for (name, content, message) in [
        ("empty.zip", empty_zip, "is empty"),
        ("not-a-zip.zip", b"not a zip".to_vec(), "cannot read zip '"),
    ] {
        let rom = scratch(name);
        std::fs::write(&rom, content).expect("write the zip");
        hc().args(["hash", "--console", NES, &rom])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(message).and(predicate::str::contains(name)));
    }
}

#[test]
fn zip_without_a_console_is_unpacked_and_its_entry_names_the_console() {
    // Unpacked, the `.nes` entry makes NES the only plausible console; left
    // packed, the `.zip` would make it arcade.
    hc().args(["hash", &fixture("nes/Zooming-Secretary.zip")])
        .assert()
        .success()
        .stdout(format!(
            "{ZOOMING_SECRETARY}\t{NES}\tNintendo Entertainment System\n"
        ));
}

#[test]
fn arcade_zip_in_an_fbneo_subsystem_folder_hashes_the_folder_with_its_name() {
    // rcheevos prefixes the name with folders FBNeo loads subsystems from,
    // such as `nes/`. MD5 of "nes_Zooming-Secretary".
    hc().args([
        "hash",
        "--console",
        ARCADE,
        &fixture("nes/Zooming-Secretary.zip"),
    ])
    .assert()
    .success()
    .stdout("1fa4080a3f7206d0dad34bc5e7bb1e5c\n");
}

/// Packs the first entry of a fixture zip into a `.7z` named `name` in a
/// scratch directory, returning the path of the 7z.
fn sevenzipped(rom: &str, name: &str) -> String {
    let file = std::fs::File::open(fixture(rom)).expect("open the fixture zip");
    let mut zip = zip::ZipArchive::new(file).expect("read the fixture zip");
    let entry = zip.by_index(0).expect("the fixture zip has an entry");
    let entry_name = entry.name().expect("the entry has a name").into_owned();
    let path = scratch(name);
    let mut archive = sevenz_rust2::ArchiveWriter::create(&path).expect("create the 7z");
    archive
        .push_archive_entry(
            sevenz_rust2::ArchiveEntry::new_file(&entry_name),
            Some(entry),
        )
        .expect("pack the ROM into the 7z");
    archive.finish().expect("finish the 7z");
    path
}

#[test]
fn seven_zipped_rom_hashes_as_the_rom_inside() {
    // A small ROM only: compressing a large one at test time, in a debug
    // build, dominates the suite's run time.
    let rom = sevenzipped("nes/Zooming-Secretary.zip", "Zooming-Secretary.7z");
    hc().args(["hash", "--console", NES, &rom])
        .assert()
        .success()
        .stdout(format!("{ZOOMING_SECRETARY}\n"));
}

#[test]
fn seven_zip_extension_matches_in_any_letter_case() {
    let rom = sevenzipped("nes/Zooming-Secretary.zip", "Zooming-Secretary.7Z");
    hc().args(["hash", "--console", NES, &rom])
        .assert()
        .success()
        .stdout(format!("{ZOOMING_SECRETARY}\n"));
}

#[test]
fn empty_or_unreadable_7z_fails_with_empty_stdout() {
    for (name, message) in [
        ("empty.7z", "is empty"),
        ("not-a-7z.7z", "cannot read 7z '"),
    ] {
        let rom = scratch(name);
        if name == "empty.7z" {
            sevenz_rust2::ArchiveWriter::create(&rom)
                .expect("create the 7z")
                .finish()
                .expect("finish the empty 7z");
        } else {
            std::fs::write(&rom, b"not a 7z").expect("write the 7z");
        }
        hc().args(["hash", "--console", NES, &rom])
            .assert()
            .failure()
            .stdout("")
            .stderr(predicate::str::contains(message).and(predicate::str::contains(name)));
    }
}

#[test]
fn seven_zip_without_a_console_is_unpacked_and_its_entry_names_the_console() {
    // Unpacked, the `.nes` entry makes NES the only plausible console; left
    // packed, the `.7z` would make it arcade.
    let rom = sevenzipped("nes/Zooming-Secretary.zip", "no-console.7z");
    hc().args(["hash", &rom]).assert().success().stdout(format!(
        "{ZOOMING_SECRETARY}\t{NES}\tNintendo Entertainment System\n"
    ));
}

#[test]
fn arcade_7z_hashes_as_its_name_without_extension() {
    // Arcade's official method reads the 7z's name, never its content, so an
    // empty file proves the 7z reaches rcheevos without being unpacked.
    let rom = scratch("mslug.7z");
    std::fs::write(&rom, b"").expect("create an empty arcade 7z");
    hc().args(["hash", "--console", ARCADE, &rom])
        .assert()
        .success()
        // MD5 of "mslug".
        .stdout("b43c8b4ec999588c04dad79bb8bcc745\n");
}
