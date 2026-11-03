mod support;

use serde_json::json;
use std::io::{BufRead, BufReader};
use std::process::{Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use support::{API_KEY, FakeServer, Reply, Request, Sandbox, age_files, files_under};

const WITH_ACHIEVEMENTS: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NONE_PUBLISHED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const NOT_LISTED: &str = "cccccccccccccccccccccccccccccccc";
const ON_PAGE_2: &str = "dddddddddddddddddddddddddddddddd";
const ON_PAGE_3: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const FETCH_SYSTEM: [&str; 4] = ["__fetch-system", "--format", "1", "7"];
const EXIT_REJECTED: i32 = 3;
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

struct Game {
    id: u32,
    title: &'static str,
    achievements: u32,
    hash: &'static str,
}

/// A one-page response in the shape the OpenAPI document gives for `/games`.
fn games_page(games: &[Game]) -> String {
    page_of(games, 1, 1)
}

/// One page of a longer list, otherwise as [`games_page`].
fn page_of(games: &[Game], current_page: u32, last_page: u32) -> String {
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
        "meta": {"page": {"currentPage": current_page, "lastPage": last_page, "perPage": 50}}
    })
    .to_string()
}

fn two_games() -> String {
    games_page(&[
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
    ])
}

/// [`two_games`] after achievements were published for the second game, and a
/// third game was added with some of its own.
fn newly_published_games() -> String {
    games_page(&[
        Game {
            id: 1,
            title: "Has Cheevos",
            achievements: 42,
            hash: WITH_ACHIEVEMENTS,
        },
        Game {
            id: 2,
            title: "No Cheevos",
            achievements: 7,
            hash: NONE_PUBLISHED,
        },
        Game {
            id: 3,
            title: "Brand New",
            achievements: 9,
            hash: NOT_LISTED,
        },
    ])
}

/// A server that lists the same two games for every console.
fn two_game_sandbox() -> Sandbox {
    Sandbox::new(FakeServer::start(|_| Reply::ok(two_games())))
}

/// A server that lists [`two_games`] once and then answers with `later`.
fn sandbox_changing_to(later: impl Fn() -> Reply + Send + 'static) -> Sandbox {
    let first = AtomicBool::new(true);
    Sandbox::new(FakeServer::start(move |_| {
        if first.swap(false, Ordering::Relaxed) {
            return Reply::ok(two_games());
        }
        later()
    }))
}

/// A run that caches [`two_games`] for console 7, which is then made to look
/// `age` old.
fn cache_list_aged(sandbox: &Sandbox, age: Duration) {
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    age_files(&sandbox.xdg_cache(), age);
}

/// A server capping pages at one game, with three games in the list.
fn three_page_sandbox() -> Sandbox {
    Sandbox::new(FakeServer::start(|request| {
        Reply::ok(three_page_list(page_number(request)))
    }))
}

/// A server answering with a 429 once and then with a one-page list.
fn once_rate_limited_sandbox(retry_after: &'static str) -> Sandbox {
    let limited = AtomicBool::new(true);
    Sandbox::new(FakeServer::start(move |_| {
        if limited.swap(false, Ordering::Relaxed) {
            return Reply::rate_limited(retry_after);
        }
        Reply::ok(games_page(&[Game {
            id: 1,
            title: "Has Cheevos",
            achievements: 42,
            hash: WITH_ACHIEVEMENTS,
        }]))
    }))
}

fn three_page_list(page: u32) -> String {
    let (id, hash) = match page {
        1 => (1, WITH_ACHIEVEMENTS),
        2 => (2, ON_PAGE_2),
        3 => (3, ON_PAGE_3),
        _ => panic!("request for page {page}"),
    };
    page_of(
        &[Game {
            id,
            title: "Paged",
            achievements: 1,
            hash,
        }],
        page,
        3,
    )
}

fn page_number(request: &Request) -> u32 {
    request
        .query
        .iter()
        .find(|(name, _)| name == "page[number]")
        .map_or(1, |(_, value)| value.parse().expect("page number"))
}

fn requested_pages(sandbox: &Sandbox) -> Vec<u32> {
    sandbox.server.requests().iter().map(page_number).collect()
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
            ("page[number]", "1"),
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
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::failure(500)));
    for _ in 0..2 {
        let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
        assert_eq!(exit_code(&output), 2);
    }
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn every_page_is_fetched_in_order_when_the_server_caps_the_page_size() {
    let sandbox = three_page_sandbox();

    for hash in [WITH_ACHIEVEMENTS, ON_PAGE_2, ON_PAGE_3] {
        let output = sandbox.run(&["md5", "--console", "nes", hash]);
        assert_eq!(exit_code(&output), 0, "{hash}: {}", stderr(&output));
    }

    // The first run asks for 100 and is given 1 per page, and later runs answer
    // from the cache.
    assert_eq!(requested_pages(&sandbox), [1, 2, 3]);
}

#[test]
fn a_failure_on_a_later_page_leaves_nothing_in_the_cache_and_the_next_run_tries_again() {
    let first_try = AtomicBool::new(true);
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        let page = page_number(request);
        if page == 2 && first_try.swap(false, Ordering::Relaxed) {
            return Reply::failure(500);
        }
        Reply::ok(three_page_list(page))
    }));

    // The hash is on the first page, which was fetched before the failure.
    let failed = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    assert_eq!(exit_code(&failed), 2, "stderr: {}", stderr(&failed));
    assert_eq!(stdout(&failed), "");

    let output = sandbox.run(&["md5", "--console", "nes", ON_PAGE_3]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_pages(&sandbox), [1, 2, 1, 2, 3]);
}

#[test]
fn progress_appears_on_stderr_while_the_download_runs() {
    let (release, released) = mpsc::channel();
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        let page = page_number(request);
        // Page 2 waits until the test has seen page 1 reported, so a parent
        // that only showed progress afterwards would never get this far.
        if page == 2 && released.recv_timeout(Duration::from_secs(10)).is_err() {
            return Reply::failure(500);
        }
        Reply::ok(three_page_list(page))
    }));

    let mut child = sandbox
        .command(&["md5", "--console", "nes", ON_PAGE_3])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run hascheevos");
    let mut lines = BufReader::new(child.stderr.take().expect("stderr")).lines();

    let first = lines
        .next()
        .expect("a line of progress")
        .expect("read stderr");
    assert_eq!(first, "console 7: page 1/3");
    release.send(()).expect("release page 2");

    let rest: Vec<String> = lines.map(|line| line.expect("read stderr")).collect();
    assert_eq!(rest[..2], ["console 7: page 2/3", "console 7: page 3/3"]);

    let output = child.wait_with_output().expect("wait for hascheevos");
    assert_eq!(exit_code(&output), 0, "stderr: {rest:?}");
    assert_eq!(stdout(&output).trim(), ON_PAGE_3);
}

#[test]
fn a_cache_hit_shows_no_progress() {
    let sandbox = three_page_sandbox();
    let download = sandbox.run(&["md5", "--console", "nes", ON_PAGE_3]);
    assert!(
        stderr(&download).contains("page 3/3"),
        "{}",
        stderr(&download)
    );

    let output = sandbox.run(&["md5", "--console", "nes", ON_PAGE_3]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert!(!stderr(&output).contains("page"), "{}", stderr(&output));
}

#[test]
fn a_failed_download_with_no_cached_list_is_reported_once_as_a_failure() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::failure(500)));
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2);
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert_eq!(stderr.matches("500").count(), 1, "stderr: {stderr}");
    assert!(stderr.starts_with("error: console 7: "), "stderr: {stderr}");
    assert!(!stderr.contains("not found"), "stderr: {stderr}");
    assert!(!stderr.contains("achievements"), "stderr: {stderr}");
}

#[test]
fn a_429_on_any_page_is_retried_and_the_download_carries_on() {
    let page_2_requests = AtomicU32::new(0);
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        let page = page_number(request);
        if page == 2 && page_2_requests.fetch_add(1, Ordering::Relaxed) < 3 {
            return Reply::rate_limited("0");
        }
        Reply::ok(three_page_list(page))
    }));

    let output = sandbox.run(&["md5", "--console", "nes", ON_PAGE_3]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).contains("rate limited"),
        "{}",
        stderr(&output)
    );
    assert_eq!(requested_pages(&sandbox), [1, 2, 2, 2, 2, 3]);
}

#[test]
fn a_429_waits_as_long_as_retry_after_says() {
    let sandbox = once_rate_limited_sandbox("2");
    let start = Instant::now();
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert!(start.elapsed() >= Duration::from_secs(2));
}

#[test]
fn a_429_without_a_number_of_seconds_is_still_retried() {
    let sandbox = once_rate_limited_sandbox("Wed, 21 Oct 2026 07:28:00 GMT");
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 2);
}

/// The real server's 429 has no `Retry-After`, and its limit lasts until a
/// minute after the first request. Ignored because it takes that minute.
#[test]
#[ignore = "takes over a minute"]
fn a_429_without_retry_after_outlasts_the_servers_one_minute_limit() {
    let start = Instant::now();
    let sandbox = Sandbox::new(FakeServer::start(move |_| {
        if start.elapsed() < Duration::from_secs(60) {
            return Reply::failure(429);
        }
        Reply::ok(games_page(&[]))
    }));
    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
}

#[test]
fn a_429_without_retry_after_is_retried_soon_in_case_it_was_brief() {
    let limited = AtomicBool::new(true);
    let sandbox = Sandbox::new(FakeServer::start(move |_| {
        if limited.swap(false, Ordering::Relaxed) {
            return Reply::failure(429);
        }
        Reply::ok(games_page(&[]))
    }));
    let start = Instant::now();
    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_download_fails_when_retries_run_out_and_the_next_run_tries_again() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::rate_limited("0")));
    for run in 1..=2 {
        let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
        assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
        assert!(stderr(&output).contains("error: "), "{}", stderr(&output));
        // The first try and 3 retries.
        assert_eq!(sandbox.server.requests().len(), run * 4);
    }
}

#[test]
fn a_very_long_retry_after_fails_rather_than_leaving_the_run_stuck() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::rate_limited("86400")));
    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn a_rejected_key_has_its_own_exit_code_from_the_hidden_subcommand() {
    for status in [401, 403] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| Reply::failure(status)));
        let output = sandbox.run(&FETCH_SYSTEM);
        assert_eq!(exit_code(&output), EXIT_REJECTED, "{status}");
        assert_eq!(sandbox.server.requests().len(), 1, "{status}");
    }
}

#[test]
fn a_rejection_on_a_later_page_has_the_same_exit_code() {
    let sandbox = Sandbox::new(FakeServer::start(|request| match page_number(request) {
        1 => Reply::ok(three_page_list(1)),
        _ => Reply::failure(401),
    }));
    assert_eq!(exit_code(&sandbox.run(&FETCH_SYSTEM)), EXIT_REJECTED);
}

#[test]
fn other_failures_of_the_hidden_subcommand_exit_2() {
    for reply in [
        || Reply::failure(500),
        || Reply::failure(404),
        || Reply::ok("not json".to_owned()),
    ] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| reply()));
        assert_eq!(exit_code(&sandbox.run(&FETCH_SYSTEM)), 2);
    }
}

#[test]
fn a_rejected_key_is_reported_as_a_failure_and_not_cached() {
    for status in [401, 403] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| Reply::failure(status)));
        for run in 1..=2 {
            let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
            assert_eq!(exit_code(&output), 2, "{status}");
            assert_eq!(stdout(&output), "");
            let stderr = stderr(&output);
            assert!(stderr.starts_with("error: "), "stderr: {stderr}");
            assert!(stderr.contains("RA_API_KEY"), "stderr: {stderr}");
            assert_eq!(sandbox.server.requests().len(), run);
        }
    }
}

#[test]
fn a_console_with_no_games_has_an_empty_list() {
    // A server may well report an empty list's last page as 0.
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::ok(page_of(&[], 1, 0))));
    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
}

#[test]
fn a_response_without_a_last_page_fails_rather_than_caching_one_page() {
    let sandbox = Sandbox::new(FakeServer::start(|_| {
        Reply::ok(games_page(&[]).replace(r#""lastPage":1,"#, ""))
    }));
    for _ in 0..2 {
        let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);
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

#[test]
fn no_achievements_in_a_day_old_list_downloads_it_and_finds_what_was_newly_published() {
    for hash in [NOT_LISTED, NONE_PUBLISHED] {
        let sandbox = sandbox_changing_to(|| Reply::ok(newly_published_games()));
        cache_list_aged(&sandbox, DAY);

        let output = sandbox.run(&["md5", "--console", "nes", hash]);

        assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
        assert_eq!(stdout(&output).trim(), hash);
        assert_eq!(sandbox.server.requests().len(), 2, "{hash}");
    }
}

#[test]
fn a_game_with_achievements_never_downloads_however_old_the_list_is() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, 10 * 365 * DAY);

    let output = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn no_achievements_in_a_list_under_a_day_old_makes_no_request() {
    for hash in [NOT_LISTED, NONE_PUBLISHED] {
        let sandbox = two_game_sandbox();
        cache_list_aged(&sandbox, DAY - Duration::from_secs(60 * 60));

        let output = sandbox.run(&["md5", "--console", "nes", hash]);

        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
        assert_eq!(sandbox.server.requests().len(), 1, "{hash}");
    }
}

#[test]
fn reading_a_list_over_a_day_old_leaves_it_on_disk() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, 2 * DAY);
    let files = files_under(&sandbox.xdg_cache());

    sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);

    assert_eq!(files_under(&sandbox.xdg_cache()), files);
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn a_download_resets_the_age_even_when_the_list_is_unchanged() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, 2 * DAY);

    for _ in 0..2 {
        let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);
        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    }

    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn an_update_shows_progress_and_a_list_still_lacking_the_hash_exits_1() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert_eq!(stderr.matches("console 7: page 1/1").count(), 1, "{stderr}");
    assert!(!stderr.contains("warning"), "{stderr}");
}

#[test]
fn a_failed_update_warns_keeps_the_cached_answer_and_exits_2() {
    let sandbox = sandbox_changing_to(|| Reply::failure(500));
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    let stderr = stderr(&output);
    assert!(stderr.starts_with("warning: console 7: "), "{stderr}");
    assert_eq!(stderr.matches("500").count(), 1, "{stderr}");
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    // Not retried in the same run.
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_failed_update_keeps_the_old_list_and_its_age() {
    let sandbox = sandbox_changing_to(|| Reply::failure(500));
    cache_list_aged(&sandbox, DAY);

    sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    let kept = sandbox.run(&["md5", "--console", "nes", WITH_ACHIEVEMENTS]);
    assert_eq!(exit_code(&kept), 0, "stderr: {}", stderr(&kept));
    assert_eq!(sandbox.server.requests().len(), 2);

    // Still due, so the next "no achievements" result tries again.
    sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);
    assert_eq!(sandbox.server.requests().len(), 3);
}

#[test]
fn a_rejected_update_is_a_failed_update() {
    let sandbox = sandbox_changing_to(|| Reply::failure(401));
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("warning: "), "{stderr}");
    assert!(stderr.contains("RA_API_KEY"), "{stderr}");
    assert!(stderr.contains("hash not found"), "{stderr}");
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn an_update_due_without_a_key_says_where_to_get_one_and_exits_2() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, DAY);

    let output = sandbox
        .command(&["md5", "--console", "nes", NOT_LISTED])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert!(
        stderr.contains("https://retroachievements.org/settings"),
        "{stderr}"
    );
    assert!(stderr.contains("hash not found"), "{stderr}");
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn refresh_downloads_a_new_list_whatever_the_result() {
    for hash in [WITH_ACHIEVEMENTS, NONE_PUBLISHED, NOT_LISTED] {
        let sandbox = sandbox_changing_to(|| Reply::ok(newly_published_games()));
        cache_list_aged(&sandbox, Duration::ZERO);

        let output = sandbox.run(&["md5", "--console", "nes", "--refresh", hash]);

        assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
        assert_eq!(sandbox.server.requests().len(), 2, "{hash}");
    }
}

#[test]
fn refresh_finds_out_that_a_game_lost_its_achievements() {
    let sandbox = sandbox_changing_to(|| Reply::ok(games_page(&[])));
    cache_list_aged(&sandbox, Duration::ZERO);

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn refresh_of_a_day_old_list_downloads_it_only_once() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", NOT_LISTED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn refresh_with_nothing_cached_downloads_only_once() {
    let sandbox = two_game_sandbox();

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn a_failed_refresh_keeps_the_cached_answer_and_exits_2() {
    let sandbox = sandbox_changing_to(|| Reply::failure(500));
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    assert!(
        stderr(&output).starts_with("warning: "),
        "{}",
        stderr(&output)
    );
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_failed_refresh_is_not_retried_by_the_refresh_rule() {
    let sandbox = sandbox_changing_to(|| Reply::failure(500));
    cache_list_aged(&sandbox, DAY);

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", NOT_LISTED]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(
        stderr(&output).matches("warning").count(),
        1,
        "{}",
        stderr(&output)
    );
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_failed_refresh_with_nothing_cached_is_tried_once_and_is_an_error() {
    let sandbox = Sandbox::new(FakeServer::start(|_| Reply::failure(500)));

    let output = sandbox.run(&["md5", "--console", "nes", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert!(
        stderr(&output).starts_with("error: "),
        "{}",
        stderr(&output)
    );
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn an_update_replaces_the_old_list_on_disk() {
    let sandbox = sandbox_changing_to(|| Reply::ok(newly_published_games()));
    cache_list_aged(&sandbox, DAY);
    let files = files_under(&sandbox.xdg_cache()).len();

    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(files_under(&sandbox.xdg_cache()).len(), files);

    // The new list is the one kept.
    let output = sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn an_update_leaves_other_consoles_lists_alone() {
    let sandbox = two_game_sandbox();
    sandbox.run(&["md5", "--console", "snes", WITH_ACHIEVEMENTS]);
    cache_list_aged(&sandbox, DAY);
    assert_eq!(sandbox.server.requests().len(), 2);

    sandbox.run(&["md5", "--console", "nes", NOT_LISTED]);
    assert_eq!(sandbox.server.requests().len(), 3);

    let output = sandbox.run(&["md5", "--console", "snes", WITH_ACHIEVEMENTS]);
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 3);
}

/// Another run may have written such a file and not yet linked it to its key,
/// which is indistinguishable from a replaced list.
#[test]
fn an_update_leaves_a_list_written_moments_ago() {
    let sandbox = two_game_sandbox();
    cache_list_aged(&sandbox, Duration::ZERO);
    let files = files_under(&sandbox.xdg_cache()).len();

    sandbox.run(&["md5", "--console", "nes", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(files_under(&sandbox.xdg_cache()).len(), files + 1);
}
