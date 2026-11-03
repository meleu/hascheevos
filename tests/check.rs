mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use support::{FakeServer, Reply, Request, Sandbox, age_files, files_under};

// From RAHasher 1.8.4, like the expected hashes in the CLI tests.
const SNES_SFC: &str = "6556112372898c69e1de0bf689d8db26";
const CHRISTMAS_CRAZE: &str = "0829ebfde80e7a9673fe9966a8d47b0f";
const CHRISTMAS_CRAZE_ARCADE: &str = "b7d59e4f4dff40e10f716433ccec74fa";
const BOTH_PSX: &str = "090500155aa193ff78586fa532a36023";
const BOTH_SEGA_CD: &str = "206d293cdd4e514fffdd4057d7f7c9fa";
// From md5sum.
const SMALL_FILE: &str = "351f58718c9ef422df35ec121ac53c3f";

const SFC_ROM: &str = "testdata/snes/FakeHeaderedROM.sfc";
const SNES_ZIP: &str = "testdata/snes/Christmas-Craze.zip";
const SNES_7Z: &str = "testdata/snes/FakeHeaderedROM.7z";
const BOTH_CUE: &str = "testdata/disc/both.cue";

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

const MEGA_DRIVE: u32 = 1;
const SNES: u32 = 3;
const NES: u32 = 7;
const SEGA_CD: u32 = 9;
const PSX: u32 = 12;
const ARCADE: u32 = 27;
const MSX: u32 = 29;
const AMSTRAD: u32 = 37;
const APPLE_II: u32 = 38;
const SATURN: u32 = 39;
const ZX_SPECTRUM: u32 = 59;

struct Game {
    id: u32,
    title: &'static str,
    achievements: u32,
    hash: &'static str,
}

fn has_cheevos(hash: &'static str) -> Game {
    Game {
        id: 1,
        title: "Has Cheevos",
        achievements: 42,
        hash,
    }
}

/// A one-page response in the shape the OpenAPI document gives for `/games`.
fn games_page(games: &[Game]) -> String {
    marked_games_page(games, "compatible")
}

/// As [`games_page`], with the hashes all marked `compatibility`.
fn marked_games_page(games: &[Game], compatibility: &str) -> String {
    let data: Vec<_> = games
        .iter()
        .map(|game| {
            json!({
                "type": "games",
                "id": game.id.to_string(),
                "attributes": {"title": game.title, "achievementsPublished": game.achievements},
                "relationships": {
                    "hashes": {"data": [{"type": "game-hashes", "id": format!("h{}", game.id)}]}
                }
            })
        })
        .collect();
    let included: Vec<_> = games
        .iter()
        .map(|game| {
            json!({
                "type": "game-hashes",
                "id": format!("h{}", game.id),
                "attributes": {"raMd5": game.hash, "compatibility": compatibility}
            })
        })
        .collect();
    json!({
        "jsonapi": {"version": "1.0"},
        "data": data,
        "included": included,
        "meta": {"page": {"currentPage": 1, "lastPage": 1, "perPage": 100}}
    })
    .to_string()
}

fn requested_console(request: &Request) -> u32 {
    request
        .query
        .iter()
        .find(|(name, _)| name == "filter[systemId]")
        .map(|(_, value)| value.parse().expect("console ID"))
        .expect("a request for a console's list")
}

/// A server listing one game for `console` and nothing for the others.
fn sandbox_listing(console: u32, game: Game) -> Sandbox {
    sandbox_publishing(0, console, game)
}

/// A server listing one game for `console`, with its hash marked
/// `compatibility`, and nothing for the others.
fn sandbox_marking(console: u32, game: Game, compatibility: &'static str) -> Sandbox {
    let list = marked_games_page(&[game], compatibility);
    let empty = games_page(&[]);
    Sandbox::new(FakeServer::start(move |request| {
        if requested_console(request) == console {
            Reply::ok(list.clone())
        } else {
            Reply::ok(empty.clone())
        }
    }))
}

fn requested_consoles(sandbox: &Sandbox) -> Vec<u32> {
    sandbox
        .server
        .requests()
        .iter()
        .map(requested_console)
        .collect()
}

/// A server that lists nothing for the first `first_requests` requests, and
/// then `later` for `console`.
fn sandbox_publishing(first_requests: u32, console: u32, later: Game) -> Sandbox {
    let list = games_page(&[later]);
    let empty = games_page(&[]);
    let requests = AtomicU32::new(0);
    Sandbox::new(FakeServer::start(move |request| {
        let earlier = requests.fetch_add(1, Ordering::Relaxed);
        if earlier >= first_requests && requested_console(request) == console {
            Reply::ok(list.clone())
        } else {
            Reply::ok(empty.clone())
        }
    }))
}

/// A server that answers the first request with `first` and later ones with
/// `later`.
fn sandbox_answering(first: Vec<Game>, later: impl Fn() -> Reply + Send + 'static) -> Sandbox {
    let first = games_page(&first);
    let requests = AtomicU32::new(0);
    Sandbox::new(FakeServer::start(move |_| {
        if requests.fetch_add(1, Ordering::Relaxed) == 0 {
            return Reply::ok(first.clone());
        }
        later()
    }))
}

/// A run that caches the lists `args` needs, which are then made to look old
/// enough for the refresh rule.
fn cache_lists_aged(sandbox: &Sandbox, args: &[&str]) {
    sandbox.run(args);
    age_files(&sandbox.xdg_cache(), DAY);
}

fn no_cheevos(hash: &'static str) -> Game {
    Game {
        id: 2,
        title: "No Cheevos",
        achievements: 0,
        hash,
    }
}

/// A file whose hash is [`SMALL_FILE`] for the consoles that hash a whole file.
fn small_rom(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, "hascheevos").unwrap();
    path
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_rom_with_achievements_has_its_path_on_stdout_and_its_title_on_stderr() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
    let stderr = stderr(&output);
    assert!(stderr.contains("Has Cheevos"), "{stderr}");
    assert!(stderr.contains("42 achievements"), "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [SNES]);
}

#[test]
fn a_rom_without_published_achievements_exits_1_with_nothing_on_stdout() {
    let sandbox = sandbox_listing(SNES, no_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
}

#[test]
fn a_rom_missing_from_the_list_exits_1() {
    let sandbox = sandbox_listing(NES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("hash not found"));
}

#[test]
fn one_rom_with_achievements_among_others_exits_0() {
    let sandbox = sandbox_listing(SNES, has_cheevos(CHRISTMAS_CRAZE));

    let output = sandbox.run(&["check", SFC_ROM, SNES_ZIP]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SNES_ZIP}\n"));
}

#[test]
fn a_seven_zip_rom_is_checked_by_the_rom_inside() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", SNES_7Z]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SNES_7Z}\n"));
}

#[test]
fn a_console_is_downloaded_once_for_many_roms() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    sandbox.run(&["check", SFC_ROM, SNES_ZIP, SFC_ROM]);

    assert_eq!(requested_consoles(&sandbox), [SNES]);
}

#[test]
fn an_ambiguous_extension_tries_candidates_until_one_matches() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let sandbox = sandbox_listing(AMSTRAD, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", rom.display()));
    assert_eq!(requested_consoles(&sandbox), [MSX, AMSTRAD]);
}

#[test]
fn every_candidate_is_tried_when_none_matches() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let sandbox = sandbox_listing(NES, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(
        requested_consoles(&sandbox),
        [MSX, AMSTRAD, ZX_SPECTRUM, APPLE_II]
    );
}

#[test]
fn a_game_without_achievements_on_one_candidate_ends_the_search() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let msx = games_page(&[no_cheevos(SMALL_FILE)]);
    let others = games_page(&[has_cheevos(SMALL_FILE)]);
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        Reply::ok(match requested_console(request) {
            MSX => msx.clone(),
            _ => others.clone(),
        })
    }));

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [MSX]);
}

#[test]
fn a_match_without_achievements_in_an_old_list_is_looked_up_again() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let list = games_page(&[has_cheevos(SMALL_FILE)]);
    let sandbox = sandbox_answering(vec![no_cheevos(SMALL_FILE)], move || {
        Reply::ok(list.clone())
    });
    cache_lists_aged(&sandbox, &["check", text(&rom)]);

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [MSX, MSX]);
}

#[test]
fn a_disc_is_looked_up_in_both_sega_cd_and_saturn() {
    let sandbox = sandbox_listing(SATURN, has_cheevos(BOTH_SEGA_CD));

    let output = sandbox.run(&["check", BOTH_CUE]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [PSX, SEGA_CD, SATURN]);
}

#[test]
fn later_candidates_are_not_consulted_after_a_match() {
    let sandbox = sandbox_listing(PSX, has_cheevos(BOTH_PSX));

    let output = sandbox.run(&["check", BOTH_CUE]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [PSX]);
}

#[test]
fn a_named_console_limits_the_search_to_it() {
    let sandbox = sandbox_listing(SEGA_CD, has_cheevos(BOTH_SEGA_CD));

    let output = sandbox.run(&["check", "--console", "segacd", BOTH_CUE]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [SEGA_CD]);
}

#[test]
fn a_named_arcade_console_uses_the_filename_hash() {
    let sandbox = sandbox_listing(ARCADE, has_cheevos(CHRISTMAS_CRAZE_ARCADE));

    let output = sandbox.run(&["check", "--console", "arcade", SNES_ZIP]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SNES_ZIP}\n"));
    assert_eq!(requested_consoles(&sandbox), [ARCADE]);
}

#[test]
fn a_path_that_is_not_utf8_reaches_stdout_unchanged() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let dir = tempfile::tempdir().unwrap();
    let rom = dir.path().join(OsStr::from_bytes(b"caf\xe9.bin"));
    std::fs::write(&rom, "hascheevos").unwrap();
    let sandbox = sandbox_listing(MEGA_DRIVE, has_cheevos(SMALL_FILE));

    let output = sandbox
        .command(&["check"])
        .arg(&rom)
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    let mut expected = rom.into_os_string().into_vec();
    expected.push(b'\n');
    assert_eq!(output.stdout, expected);
}

#[test]
fn an_old_list_is_downloaded_after_no_achievements_and_the_rom_looked_up_again() {
    let sandbox = sandbox_publishing(1, SNES, has_cheevos(SNES_SFC));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn a_recent_list_is_not_downloaded_after_no_achievements() {
    let sandbox = sandbox_listing(SNES, no_cheevos(SNES_SFC));
    sandbox.run(&["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [SNES]);
}

#[test]
fn an_old_list_is_not_downloaded_when_it_has_achievements_for_the_rom() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [SNES]);
}

#[test]
fn an_old_list_is_not_downloaded_when_another_candidate_has_the_game() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let sandbox = sandbox_listing(AMSTRAD, has_cheevos(SMALL_FILE));
    cache_lists_aged(&sandbox, &["check", "--console", "msx", text(&rom)]);

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [MSX, AMSTRAD]);
}

#[test]
fn each_old_list_consulted_is_downloaded_until_one_has_the_game() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let sandbox = sandbox_publishing(4, AMSTRAD, has_cheevos(SMALL_FILE));
    cache_lists_aged(&sandbox, &["check", text(&rom)]);

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(
        requested_consoles(&sandbox),
        [MSX, AMSTRAD, ZX_SPECTRUM, APPLE_II, MSX, AMSTRAD]
    );
}

#[test]
fn the_second_lookup_reuses_the_hashes_already_calculated() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.bin");
    let removed = rom.clone();
    let list = games_page(&[has_cheevos(SMALL_FILE)]);
    let sandbox = sandbox_answering(Vec::new(), move || {
        // Hashing the ROM again would now fail.
        std::fs::remove_file(&removed).unwrap();
        Reply::ok(list.clone())
    });
    cache_lists_aged(&sandbox, &["check", text(&rom)]);

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", rom.display()));
}

#[test]
fn a_failed_update_warns_and_the_cached_answer_stands_for_every_rom() {
    let sandbox = sandbox_answering(vec![no_cheevos(SNES_SFC)], || Reply::failure(500));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM, SNES_ZIP]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert_eq!(stderr.matches("warning: ").count(), 2, "{stderr}");
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn a_failed_refresh_warns_once_for_a_rom_with_an_old_list() {
    let sandbox = sandbox_answering(vec![no_cheevos(SNES_SFC)], || Reply::failure(500));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", "--refresh", SFC_ROM]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert_eq!(stderr.matches("warning: ").count(), 1, "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn a_rejected_update_gives_the_cached_answer_and_stops_the_run() {
    let sandbox = sandbox_answering(vec![no_cheevos(SNES_SFC)], || Reply::failure(401));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM, BOTH_CUE]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert!(stderr.contains("warning: "), "{stderr}");
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn refresh_downloads_a_recent_list_even_for_a_rom_with_achievements() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));
    sandbox.run(&["check", SFC_ROM]);

    let output = sandbox.run(&["check", "--refresh", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn refresh_answers_from_the_new_list() {
    let list = games_page(&[no_cheevos(SNES_SFC)]);
    let sandbox = sandbox_answering(vec![has_cheevos(SNES_SFC)], move || Reply::ok(list.clone()));
    sandbox.run(&["check", SFC_ROM]);

    let output = sandbox.run(&["check", "--refresh", SFC_ROM]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn refresh_downloads_a_console_once_for_many_roms() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));
    sandbox.run(&["check", SFC_ROM]);

    sandbox.run(&["check", "--refresh", SFC_ROM, SNES_ZIP, SFC_ROM]);

    assert_eq!(requested_consoles(&sandbox), [SNES, SNES]);
}

#[test]
fn refresh_stops_at_the_first_match() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let sandbox = sandbox_listing(AMSTRAD, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", "--refresh", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_consoles(&sandbox), [MSX, AMSTRAD]);
}

#[test]
fn an_excluded_hash_is_left_out_with_a_warning_naming_the_file() {
    for (compatibility, reason) in [
        ("incompatible", "incompatible"),
        ("patch-required", "patch"),
        ("some-new-value", "some-new-value"),
    ] {
        let sandbox = sandbox_marking(SNES, has_cheevos(SNES_SFC), compatibility);

        let output = sandbox.run(&["check", SFC_ROM]);

        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
        assert_eq!(stdout(&output), "");
        let warning = stderr(&output)
            .lines()
            .find(|line| line.starts_with("warning: "))
            .expect("a warning")
            .to_owned();
        assert!(warning.contains(SFC_ROM), "{warning}");
        assert!(warning.contains(reason), "{warning}");
    }
}

#[test]
fn an_untested_hash_counts_without_a_warning() {
    let sandbox = sandbox_marking(SNES, has_cheevos(SNES_SFC), "untested");

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), SFC_ROM);
    assert!(
        !stderr(&output).contains("warning: "),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_excluded_rom_does_not_stop_another_rom_counting() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.nes");
    let sandbox = Sandbox::new(FakeServer::start(|request| {
        Reply::ok(if requested_console(request) == SNES {
            marked_games_page(&[has_cheevos(SNES_SFC)], "incompatible")
        } else {
            games_page(&[has_cheevos(SMALL_FILE)])
        })
    }));

    let output = sandbox.run(&["check", SFC_ROM, text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), text(&rom));
}

#[test]
fn an_excluded_hash_downloads_an_old_list_and_counts_once_it_is_compatible() {
    let requests = AtomicU32::new(0);
    let sandbox = Sandbox::new(FakeServer::start(move |_| {
        let compatibility = if requests.fetch_add(1, Ordering::Relaxed) == 0 {
            "patch-required"
        } else {
            "compatible"
        };
        Reply::ok(marked_games_page(&[has_cheevos(SNES_SFC)], compatibility))
    }));
    cache_lists_aged(&sandbox, &["check", SFC_ROM]);

    let output = sandbox.run(&["check", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), SFC_ROM);
    assert!(
        !stderr(&output).contains("warning: "),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_console_note_does_not_appear_on_an_excluded_hash() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.bin");
    for compatibility in ["incompatible", "patch-required"] {
        let sandbox = sandbox_marking(MEGA_DRIVE, has_cheevos(SMALL_FILE), compatibility);

        let output = sandbox.run(&["check", text(&rom)]);

        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
        assert!(!stderr(&output).contains("note: "), "{}", stderr(&output));
    }
}

#[test]
fn an_excluded_hash_with_none_published_warns_and_skips_the_console_note() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.bin");
    let sandbox = sandbox_marking(MEGA_DRIVE, no_cheevos(SMALL_FILE), "incompatible");

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert!(stderr.contains("warning: "), "{stderr}");
    assert!(stderr.contains("incompatible"), "{stderr}");
    assert!(!stderr.contains("note: "), "{stderr}");
}

#[test]
fn a_run_where_every_rom_is_excluded_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let first = small_rom(dir.path(), "first.nes");
    let second = small_rom(dir.path(), "second.nes");
    let sandbox = sandbox_marking(NES, has_cheevos(SMALL_FILE), "incompatible");

    let output = sandbox.run(&["check", text(&first), text(&second)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_shared_extension_without_achievements_gets_a_note_suggesting_console() {
    let dir = tempfile::tempdir().unwrap();
    // rcheevos needs a DS header, which it accepts as all zeros.
    std::fs::write(dir.path().join("game.nds"), [0; 4096]).unwrap();
    small_rom(dir.path(), "game.bin");
    small_rom(dir.path(), "game.ROM");
    small_rom(dir.path(), "game.tap");
    for name in ["game.bin", "game.ROM", "game.tap", "game.nds"] {
        let rom = dir.path().join(name);
        let sandbox = sandbox_listing(NES, has_cheevos(SMALL_FILE));

        let output = sandbox.run(&["check", text(&rom)]);

        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
        let stderr = stderr(&output);
        assert!(stderr.contains("note: "), "{name}: {stderr}");
        assert!(stderr.contains("--console"), "{name}: {stderr}");
    }
}

#[test]
fn a_shared_extension_inside_a_zip_gets_the_note_too() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("game.zip");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    archive
        .start_file("game.bin", zip::write::SimpleFileOptions::default())
        .unwrap();
    std::io::Write::write_all(&mut archive, b"hascheevos").unwrap();
    archive.finish().unwrap();
    let sandbox = sandbox_listing(NES, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", text(&path)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("--console"));
}

#[test]
fn the_console_note_does_not_appear_on_a_positive_result() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.bin");
    let sandbox = sandbox_listing(MEGA_DRIVE, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert!(!stderr(&output).contains("note: "));
}

#[test]
fn the_console_note_does_not_appear_with_a_named_console() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "game.bin");
    let sandbox = sandbox_listing(NES, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", "--console", "megadrive", text(&rom)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert!(!stderr(&output).contains("note: "));
}

#[test]
fn an_unrecognised_extension_is_skipped_with_a_note() {
    let dir = tempfile::tempdir().unwrap();
    let file = small_rom(dir.path(), "gamelist.xml");
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", text(&file)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("note: "));
    assert!(requested_consoles(&sandbox).is_empty());

    let output = sandbox.run(&["check", text(&file), SFC_ROM]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
}

#[test]
fn with_a_named_console_an_unrecognised_extension_is_hashed_as_usual() {
    let dir = tempfile::tempdir().unwrap();
    let file = small_rom(dir.path(), "game.xyz");
    let sandbox = sandbox_listing(NES, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", "--console", "nes", text(&file)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", file.display()));
}

#[test]
fn a_chd_is_reported_as_unsupported_and_counts_as_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let file = small_rom(dir.path(), "game.chd");
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", text(&file), SFC_ROM]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
    assert!(stderr(&output).contains("CHD files are not supported"));
}

#[test]
fn a_missing_file_or_unreadable_archive_fails_and_the_run_continues() {
    let dir = tempfile::tempdir().unwrap();
    let bad_zip = small_rom(dir.path(), "bad.zip");
    let bad_7z = small_rom(dir.path(), "bad.7z");
    let missing = dir.path().join("missing.sfc");
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&[
        "check",
        text(&missing),
        text(&bad_zip),
        text(&bad_7z),
        SFC_ROM,
    ]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
    let stderr = stderr(&output);
    assert!(stderr.contains("missing.sfc: no such file"), "{stderr}");
    assert!(
        stderr.contains("error: ") && stderr.contains("bad.zip"),
        "{stderr}"
    );
    assert!(
        stderr.contains("error: ") && stderr.contains("bad.7z"),
        "{stderr}"
    );
}

#[test]
fn a_file_that_produces_no_hash_counts_as_a_failure() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", "--console", "3ds", "testdata/disc/encrypted.3ds"]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert!(
        stderr.contains("normal keys callback was not set"),
        "{stderr}"
    );
    assert!(stderr.contains("no hash produced"), "{stderr}");
    assert!(requested_consoles(&sandbox).is_empty());
}

#[test]
fn a_failed_download_is_reported_per_rom_but_only_tried_once() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::failure(500)));

    let output = sandbox.run(&["check", SFC_ROM, SNES_ZIP]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert!(stderr.contains(&format!("error: {SFC_ROM}: ")), "{stderr}");
    assert!(stderr.contains(&format!("error: {SNES_ZIP}: ")), "{stderr}");
    assert_eq!(requested_consoles(&sandbox), [SNES]);
}

#[test]
fn a_failed_candidate_does_not_end_the_search() {
    let dir = tempfile::tempdir().unwrap();
    let rom = small_rom(dir.path(), "disk.dsk");
    let list = games_page(&[has_cheevos(SMALL_FILE)]);
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        match requested_console(request) {
            MSX => Reply::failure(500),
            _ => Reply::ok(list.clone()),
        }
    }));

    let output = sandbox.run(&["check", text(&rom)]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{}\n", rom.display()));
}

#[test]
fn a_rejected_key_stops_the_run() {
    for status in [401, 403] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| Reply::failure(status)));

        let output = sandbox.run(&["check", SFC_ROM, BOTH_CUE]);

        assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
        let stderr = stderr(&output);
        assert_eq!(stderr.matches("rejected RA_API_KEY").count(), 1, "{stderr}");
        assert_eq!(requested_consoles(&sandbox), [SNES]);
    }
}

#[test]
fn a_missing_key_gives_one_helpful_message() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox
        .command(&["check", SFC_ROM, BOTH_CUE])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert_eq!(
        stderr.matches("RA_API_KEY is not set").count(),
        1,
        "{stderr}"
    );
    assert!(
        stderr.contains("retroachievements.org/settings"),
        "{stderr}"
    );
    assert!(requested_consoles(&sandbox).is_empty());
}

#[test]
fn tab_output_gives_a_rom_with_achievements_its_game_id_and_an_x() {
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", "--tab-output", SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\t1\tx\n"));
}

#[test]
fn tab_output_leaves_the_x_blank_when_there_are_no_achievements() {
    let dir = tempfile::tempdir().unwrap();
    let not_found = small_rom(dir.path(), "not-found.nes");
    let sandbox = sandbox_listing(SNES, no_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", "--tab-output", SFC_ROM, text(&not_found)]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("{SFC_ROM}\t2\t\n{}\t\t\n", text(&not_found))
    );
}

#[test]
fn tab_output_leaves_the_x_blank_on_an_excluded_hash() {
    let sandbox = sandbox_marking(SNES, has_cheevos(SNES_SFC), "incompatible");

    let output = sandbox.run(&["check", "--tab-output", SFC_ROM]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\t1\t\n"));
}

#[test]
fn tab_output_has_no_row_for_a_failed_or_skipped_rom() {
    let dir = tempfile::tempdir().unwrap();
    let skipped = small_rom(dir.path(), "gamelist.xml");
    let missing = dir.path().join("missing.sfc");
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&[
        "check",
        "--tab-output",
        text(&missing),
        text(&skipped),
        SFC_ROM,
    ]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\t1\tx\n"));
}

#[test]
fn tab_output_names_a_rom_without_the_tags_in_its_file_name() {
    let dir = tempfile::tempdir().unwrap();
    let tagged_dir = dir.path().join("roms (old)");
    std::fs::create_dir(&tagged_dir).unwrap();
    let rom = small_rom(&tagged_dir, "Some Game (USA) (Rev 1).bin");
    let sandbox = sandbox_listing(MEGA_DRIVE, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", "--tab-output", text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!("{}\t1\tx\n", text(&tagged_dir.join("Some Game")))
    );
}

#[test]
fn copy_roms_to_copies_a_rom_with_achievements_into_a_new_console_directory() {
    let dir = tempfile::tempdir().unwrap();
    let destination = dir.path().join("not/there/yet");
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", "--copy-roms-to", text(&destination), SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n"));
    assert_eq!(
        std::fs::read(destination.join("snes/FakeHeaderedROM.sfc")).unwrap(),
        std::fs::read(SFC_ROM).unwrap()
    );
}

#[test]
fn copy_roms_to_uses_the_console_that_matched() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = sandbox_listing(SATURN, has_cheevos(BOTH_SEGA_CD));

    let output = sandbox.run(&["check", "--copy-roms-to", text(dir.path()), BOTH_CUE]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(
        files_under(dir.path()),
        [dir.path().join("saturn/both.cue")]
    );
}

#[test]
fn copy_roms_to_copies_an_archive_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = sandbox_listing(SNES, has_cheevos(CHRISTMAS_CRAZE));

    let output = sandbox.run(&["check", "--copy-roms-to", text(dir.path()), SNES_ZIP]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(
        std::fs::read(dir.path().join("snes/Christmas-Craze.zip")).unwrap(),
        std::fs::read(SNES_ZIP).unwrap()
    );
}

#[test]
fn copy_roms_to_overwrites_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("snes/FakeHeaderedROM.sfc");
    std::fs::create_dir_all(existing.parent().unwrap()).unwrap();
    std::fs::write(&existing, "an older copy").unwrap();
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&["check", "--copy-roms-to", text(dir.path()), SFC_ROM]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(
        std::fs::read(&existing).unwrap(),
        std::fs::read(SFC_ROM).unwrap()
    );
}

#[test]
fn copy_roms_to_leaves_a_rom_that_is_already_there_intact() {
    let dir = tempfile::tempdir().unwrap();
    let console_dir = dir.path().join("megadrive");
    std::fs::create_dir(&console_dir).unwrap();
    let rom = small_rom(&console_dir, "game.bin");
    let sandbox = sandbox_listing(MEGA_DRIVE, has_cheevos(SMALL_FILE));

    let output = sandbox.run(&["check", "--copy-roms-to", text(dir.path()), text(&rom)]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(std::fs::read_to_string(&rom).unwrap(), "hascheevos");
}

#[test]
fn copy_roms_to_leaves_out_roms_without_achievements() {
    let dir = tempfile::tempdir().unwrap();
    let not_found = small_rom(dir.path(), "not-found.nes");
    let destination = dir.path().join("copies");
    let sandbox = sandbox_listing(SNES, no_cheevos(SNES_SFC));
    let excluded = sandbox_marking(SNES, has_cheevos(SNES_SFC), "incompatible");

    let output = sandbox.run(&[
        "check",
        "--copy-roms-to",
        text(&destination),
        SFC_ROM,
        text(&not_found),
    ]);
    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));

    let output = excluded.run(&["check", "--copy-roms-to", text(&destination), SFC_ROM]);
    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));

    assert!(!destination.exists());
}

#[test]
fn a_failed_copy_is_reported_and_the_run_continues_to_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    // A file where the first ROM's console directory has to go.
    std::fs::write(dir.path().join("snes"), "in the way").unwrap();
    let sandbox = Sandbox::new(FakeServer::start(|request| {
        Reply::ok(match requested_console(request) {
            SNES => games_page(&[has_cheevos(SNES_SFC)]),
            MEGA_DRIVE => games_page(&[has_cheevos(SMALL_FILE)]),
            _ => games_page(&[]),
        })
    }));
    let rom_dir = tempfile::tempdir().unwrap();
    let second = small_rom(rom_dir.path(), "second.bin");

    let output = sandbox.run(&[
        "check",
        "--copy-roms-to",
        text(dir.path()),
        SFC_ROM,
        text(&second),
    ]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\n{}\n", text(&second)));
    let stderr = stderr(&output);
    assert!(
        stderr.contains("error: ") && stderr.contains("FakeHeaderedROM.sfc: can't copy"),
        "{stderr}"
    );
    assert!(dir.path().join("megadrive/second.bin").exists());
}

#[test]
fn tab_output_and_copy_roms_to_work_together() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = sandbox_listing(SNES, has_cheevos(SNES_SFC));

    let output = sandbox.run(&[
        "check",
        "--tab-output",
        "--copy-roms-to",
        text(dir.path()),
        SFC_ROM,
    ]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\t1\tx\n"));
    assert!(dir.path().join("snes/FakeHeaderedROM.sfc").exists());
}

#[test]
fn tab_output_has_no_row_when_a_candidate_failed_and_the_others_lack_the_hash() {
    let sandbox = Sandbox::new(FakeServer::start(|request| {
        if requested_console(request) == PSX {
            Reply::failure(500)
        } else {
            Reply::ok(games_page(&[]))
        }
    }));

    let output = sandbox.run(&["check", "--tab-output", BOTH_CUE]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn tab_output_has_a_row_when_another_candidate_has_the_game_after_a_failure() {
    let sandbox = Sandbox::new(FakeServer::start(|request| {
        Reply::ok(match requested_console(request) {
            SEGA_CD => games_page(&[has_cheevos(BOTH_SEGA_CD)]),
            _ => return Reply::failure(500),
        })
    }));

    let output = sandbox.run(&["check", "--tab-output", BOTH_CUE]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{BOTH_CUE}\t1\tx\n"));
}

#[test]
fn tab_output_keeps_the_cached_answer_when_an_update_fails() {
    let sandbox = sandbox_answering(vec![], || Reply::failure(500));
    cache_lists_aged(&sandbox, &["check", "--console", "snes", SFC_ROM]);

    let output = sandbox.run(&["check", "--tab-output", "--console", "snes", SFC_ROM]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), format!("{SFC_ROM}\t\t\n"));
}
