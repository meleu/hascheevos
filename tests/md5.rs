mod support;

use serde_json::json;
use std::process::Output;
use std::time::Duration;
use support::{API_KEY, FakeServer, Reply, Sandbox, age_files, files_under};

const WITH_ACHIEVEMENTS: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NONE_PUBLISHED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const NOT_LISTED: &str = "cccccccccccccccccccccccccccccccc";

struct Game {
    id: u32,
    title: &'static str,
    achievements: u32,
    hash: &'static str,
}

/// A one-page response in the shape the OpenAPI document gives for `/games`.
fn games_page(games: &[Game]) -> String {
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
                "attributes": {"raMd5": game.hash, "compatibility": "compatible"}
            })
        })
        .collect();
    json!({
        "jsonapi": {"version": "1.0"},
        "data": data,
        "included": included,
        "meta": {"page": {"currentPage": 1, "lastPage": 1, "perPage": 50, "total": games.len()}}
    })
    .to_string()
}

/// A server that lists the same two games for every console.
fn two_game_sandbox() -> Sandbox {
    Sandbox::new(FakeServer::start(|_| Reply {
        status: 200,
        body: games_page(&[
            Game {
                id: 1,
                title: "Has Cheevos",
                achievements: 42,
                hash: WITH_ACHIEVEMENTS,
            },
            Game {
                id: 2,
                title: "No Cheevos",
                achievements: 0,
                hash: NONE_PUBLISHED,
            },
        ]),
    }))
}

fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("exit code")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_hash_with_published_achievements_exits_0() {
    let sandbox = two_game_sandbox();
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    assert!(
        stderr(&output).contains("Has Cheevos"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn a_hash_whose_game_has_none_published_exits_1() {
    let output = two_game_sandbox().run(&["md5", "--console", "nes", NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_hash_not_in_the_list_exits_1() {
    let output = two_game_sandbox().run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn an_uppercase_hash_still_matches() {
    let hash = WITH_ACHIEVEMENTS.to_uppercase();
    let output = two_game_sandbox().run(&["md5", "--console", "nes", &hash]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
}

#[test]
fn the_request_names_the_console_and_only_the_fields_lookups_need() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    let requests = sandbox.server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/games");
    assert_eq!(requests[0].api_key.as_deref(), Some(API_KEY));
    let mut query: Vec<(&str, &str)> = requests[0]
        .query
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    query.sort_unstable();
    assert_eq!(
        query,
        [
            ("fields[game-hashes]", "raMd5,compatibility"),
            ("fields[games]", "title,achievementsPublished,hashes"),
            ("filter[systemId]", "7"),
            ("include", "hashes"),
            ("page[size]", "100"),
        ]
    );
}

#[test]
fn a_second_run_answers_from_the_cache() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn each_console_has_its_own_list() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    sandbox.run(&["md5", "--console", "snes", WITH_ACHIEVEMENTS]);

    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn entries_live_under_xdg_cache_home() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert!(!files_under(&sandbox.xdg_cache().join("hascheevos")).is_empty());
    assert!(files_under(&sandbox.home()).is_empty());
}

#[test]
fn entries_fall_back_to_the_home_cache_directory() {
    let sandbox = two_game_sandbox();
    let unset = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .env_remove("XDG_CACHE_HOME")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&unset), 0, "stderr: {}", stderr(&unset));
    assert!(!files_under(&sandbox.home().join(".cache/hascheevos")).is_empty());

    // An empty value counts as unset, so this finds the same entry.
    let empty = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .env("XDG_CACHE_HOME", "")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&empty), 0, "stderr: {}", stderr(&empty));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn a_very_old_list_is_kept_and_still_answers() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    age_files(
        &sandbox.xdg_cache(),
        Duration::from_secs(10 * 365 * 24 * 60 * 60),
    );

    // A download for another console also has `bkt` clean up what it calls
    // stale, which is decided by the TTL the entry was written with.
    sandbox.run(&["md5", "--console", "snes", WITH_ACHIEVEMENTS]);
    assert_eq!(sandbox.server.requests().len(), 2);

    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn the_key_is_neither_part_of_the_cache_key_nor_written_to_disk() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    let output = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .env("RA_API_KEY", "another-key")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);

    for file in files_under(&sandbox.xdg_cache()) {
        let contents = std::fs::read(&file).expect("read cache file");
        assert!(
            !contents
                .windows(API_KEY.len())
                .any(|window| window == API_KEY.as_bytes()),
            "key found in {}",
            file.display()
        );
    }
}

#[test]
fn a_missing_key_says_where_to_get_one_and_exits_2() {
    for key in [None, Some("")] {
        let sandbox = two_game_sandbox();
        let mut command = sandbox.command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
        match key {
            Some(key) => command.env("RA_API_KEY", key),
            None => command.env_remove("RA_API_KEY"),
        };
        let output = command.output().expect("run hascheevos");

        assert_eq!(exit_code(&output), 2);
        assert_eq!(stdout(&output), "");
        assert!(
            stderr(&output).contains("https://retroachievements.org/settings"),
            "stderr: {}",
            stderr(&output)
        );
        assert!(sandbox.server.requests().is_empty());
    }
}

#[test]
fn a_cached_list_answers_without_a_key() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    let output = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
}

#[test]
fn hash_works_without_a_key() {
    let output = two_game_sandbox()
        .command(&[
            "hash",
            "--console",
            "snes",
            "testdata/snes/FakeHeaderedROM.sfc",
        ])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
}

#[test]
fn the_hidden_subcommand_requires_a_format_version() {
    let sandbox = two_game_sandbox();
    assert_eq!(exit_code(&sandbox.run(&["__fetch-system", "7"])), 2);
    assert!(sandbox.server.requests().is_empty());
}

#[test]
fn the_hidden_subcommand_stays_out_of_the_help() {
    let output = two_game_sandbox().run(&["--help"]);
    assert!(!stdout(&output).contains("__fetch-system"));
}

#[test]
fn md5_without_a_console_is_rejected() {
    let sandbox = two_game_sandbox();
    let output = sandbox.run(&["md5", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2);
    assert!(
        stderr(&output).contains("--console"),
        "stderr: {}",
        stderr(&output)
    );
    assert!(sandbox.server.requests().is_empty());
}

#[test]
fn a_failed_download_is_not_cached() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply {
        status: 500,
        body: String::new(),
    }));
    for _ in 0..2 {
        let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
        assert_eq!(exit_code(&output), 2);
    }
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_list_of_several_pages_fails_rather_than_caching_one_page() {
    let sandbox = Sandbox::new(FakeServer::start(|_| {
        let page = games_page(&[]).replace(r#""lastPage":1"#, r#""lastPage":2"#);
        Reply {
            status: 200,
            body: page,
        }
    }));
    for _ in 0..2 {
        let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
        assert_eq!(exit_code(&output), 2);
    }
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn an_empty_home_is_an_error_rather_than_a_relative_cache() {
    let sandbox = two_game_sandbox();
    let output = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .env_remove("XDG_CACHE_HOME")
        .env("HOME", "")
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert!(sandbox.server.requests().is_empty());
}

#[test]
fn a_reader_closing_the_pipe_doesnt_change_the_exit_code() {
    let sandbox = two_game_sandbox();
    let (reader, writer) = std::io::pipe().expect("create pipe");
    drop(reader);
    let output = sandbox
        .command(&["md5", "--console", "nes", WITH_ACHIEVEMENTS])
        .stdout(writer)
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
}
