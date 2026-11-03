mod support;

use serde_json::json;
use std::process::Output;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use support::{API_KEY, FakeServer, Reply, Request, Sandbox, age_files, files_under};

const WITH_ACHIEVEMENTS: &str = "1";
const NONE_PUBLISHED: &str = "2";
const UNKNOWN: &str = "3";
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// A response in the shape the OpenAPI document gives for `/games/{game}`.
fn game(id: &str, title: &str, achievements: u32) -> String {
    json!({
        "jsonapi": {"version": "1.0"},
        "data": {
            "type": "games",
            "id": id,
            "attributes": {"title": title, "achievementsPublished": achievements}
        }
    })
    .to_string()
}

fn requested_id(request: &Request) -> &str {
    request
        .path
        .strip_prefix("/games/")
        .expect("a request for one game")
}

/// A server knowing game 1, with achievements, and game 2, with none.
fn known_games(request: &Request) -> Reply {
    match requested_id(request) {
        WITH_ACHIEVEMENTS => Reply::ok(game(WITH_ACHIEVEMENTS, "Has Cheevos", 42)),
        NONE_PUBLISHED => Reply::ok(game(NONE_PUBLISHED, "No Cheevos", 0)),
        _ => Reply::failure(404),
    }
}

fn known_games_sandbox() -> Sandbox {
    Sandbox::new(FakeServer::start(known_games))
}

/// A server answering as [`known_games`] once and then with `later`.
fn sandbox_changing_to(later: impl Fn(&Request) -> Reply + Send + 'static) -> Sandbox {
    let first = AtomicBool::new(true);
    Sandbox::new(FakeServer::start(move |request| {
        if first.swap(false, Ordering::Relaxed) {
            return known_games(request);
        }
        later(request)
    }))
}

/// A run that caches `id`, which is then made to look `age` old.
fn cache_game_aged(sandbox: &Sandbox, id: &str, age: Duration) {
    sandbox.run(&["game", id]);
    age_files(&sandbox.xdg_cache(), age);
}

fn requested_ids(sandbox: &Sandbox) -> Vec<String> {
    sandbox
        .server
        .requests()
        .iter()
        .map(|request| requested_id(request).to_owned())
        .collect()
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
fn a_game_with_achievements_exits_0_and_shows_its_title() {
    let sandbox = known_games_sandbox();
    let output = sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    assert!(
        stderr(&output).contains("Has Cheevos"),
        "stderr: {}",
        stderr(&output)
    );

    let requests = sandbox.server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/games/1");
    assert_eq!(requests[0].api_key.as_deref(), Some(API_KEY));
    assert_eq!(
        requests[0].query,
        [(
            "fields[games]".to_owned(),
            "title,achievementsPublished".to_owned()
        )]
    );
}

#[test]
fn a_game_with_none_published_exits_1_and_shows_its_title() {
    let output = known_games_sandbox().run(&["game", NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert!(
        stderr(&output).contains("No Cheevos"),
        "stderr: {}",
        stderr(&output)
    );
}

#[test]
fn several_games_exit_0_if_any_has_achievements_and_only_those_are_printed() {
    let sandbox = known_games_sandbox();
    let output = sandbox.run(&["game", NONE_PUBLISHED, WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    let stderr = stderr(&output);
    assert!(stderr.contains("No Cheevos"), "stderr: {stderr}");
    assert!(stderr.contains("Has Cheevos"), "stderr: {stderr}");
}

#[test]
fn several_games_with_none_published_exit_1() {
    let sandbox = Sandbox::new(FakeServer::start(|request| {
        Reply::ok(game(requested_id(request), "No Cheevos", 0))
    }));
    let output = sandbox.run(&["game", "4", "5"]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(requested_ids(&sandbox), ["4", "5"]);
}

#[test]
fn no_ids_or_one_that_isnt_a_number_is_rejected_without_a_request() {
    let sandbox = known_games_sandbox();
    for args in [&["game"][..], &["game", "1", "mario"][..]] {
        assert_eq!(exit_code(&sandbox.run(args)), 2, "{args:?}");
    }
    assert!(sandbox.server.requests().is_empty());
}

#[test]
fn a_second_run_answers_from_the_cache() {
    for (id, code) in [(WITH_ACHIEVEMENTS, 0), (NONE_PUBLISHED, 1)] {
        let sandbox = known_games_sandbox();
        sandbox.run(&["game", id]);
        let output = sandbox.run(&["game", id]);

        assert_eq!(exit_code(&output), code, "stderr: {}", stderr(&output));
        assert_eq!(sandbox.server.requests().len(), 1, "{id}");
    }
}

#[test]
fn each_game_has_its_own_entry() {
    let sandbox = known_games_sandbox();
    sandbox.run(&["game", WITH_ACHIEVEMENTS]);
    sandbox.run(&["game", NONE_PUBLISHED]);
    let output = sandbox.run(&["game", WITH_ACHIEVEMENTS, NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(requested_ids(&sandbox), [WITH_ACHIEVEMENTS, NONE_PUBLISHED]);
}

#[test]
fn a_cached_game_answers_without_a_key() {
    let sandbox = known_games_sandbox();
    sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    let output = sandbox
        .command(&["game", WITH_ACHIEVEMENTS])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");
    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
}

#[test]
fn none_published_in_a_day_old_entry_downloads_it_and_finds_what_was_newly_published() {
    let sandbox =
        sandbox_changing_to(|request| Reply::ok(game(requested_id(request), "No Cheevos", 7)));
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY);

    let output = sandbox.run(&["game", NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), NONE_PUBLISHED);
    assert_eq!(sandbox.server.requests().len(), 2);

    // The new answer is the one kept.
    assert_eq!(exit_code(&sandbox.run(&["game", NONE_PUBLISHED])), 0);
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_game_with_achievements_never_downloads_however_old_its_entry_is() {
    let sandbox = known_games_sandbox();
    cache_game_aged(&sandbox, WITH_ACHIEVEMENTS, 10 * 365 * DAY);

    let output = sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn none_published_in_an_entry_under_a_day_old_makes_no_request() {
    let sandbox = known_games_sandbox();
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY - Duration::from_secs(60 * 60));

    let output = sandbox.run(&["game", NONE_PUBLISHED]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn a_download_resets_the_age_and_replaces_the_old_entry_on_disk() {
    let sandbox = known_games_sandbox();
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY);
    let files = files_under(&sandbox.xdg_cache()).len();

    for _ in 0..2 {
        let output = sandbox.run(&["game", NONE_PUBLISHED]);
        assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    }

    assert_eq!(sandbox.server.requests().len(), 2);
    assert_eq!(files_under(&sandbox.xdg_cache()).len(), files);
}

#[test]
fn refresh_downloads_whatever_the_result_or_age() {
    for id in [WITH_ACHIEVEMENTS, NONE_PUBLISHED] {
        let sandbox = known_games_sandbox();
        sandbox.run(&["game", id]);

        sandbox.run(&["game", "--refresh", id]);

        assert_eq!(sandbox.server.requests().len(), 2, "{id}");
    }
}

#[test]
fn refresh_finds_out_that_a_game_lost_its_achievements() {
    let sandbox =
        sandbox_changing_to(|request| Reply::ok(game(requested_id(request), "Has Cheevos", 0)));
    sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    let output = sandbox.run(&["game", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 1, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_failed_refresh_keeps_the_cached_answer_and_exits_2() {
    let sandbox = sandbox_changing_to(|_| Reply::failure(500));
    sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    let output = sandbox.run(&["game", "--refresh", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    assert!(
        stderr(&output).starts_with("warning: game 1: "),
        "{}",
        stderr(&output)
    );
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_game_is_downloaded_at_most_once_per_run() {
    let sandbox = known_games_sandbox();
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY);

    sandbox.run(&["game", "--refresh", NONE_PUBLISHED, NONE_PUBLISHED]);

    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_failed_lookup_doesnt_stop_the_remaining_ids_and_exits_2() {
    let sandbox = known_games_sandbox();
    let output = sandbox.run(&["game", UNKNOWN, WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    let stderr = stderr(&output);
    assert!(stderr.starts_with("error: game 3: "), "stderr: {stderr}");
    assert!(stderr.contains("Has Cheevos"), "stderr: {stderr}");
}

#[test]
fn a_failed_lookup_is_not_cached() {
    for reply in [
        || Reply::failure(404),
        || Reply::failure(500),
        || Reply::ok("not json".to_owned()),
        // Valid JSON, but without what the answer depends on.
        || Reply::ok(r#"{"data": {"type": "games", "id": "3", "attributes": {}}}"#.to_owned()),
    ] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| reply()));
        for _ in 0..2 {
            let output = sandbox.run(&["game", UNKNOWN]);
            assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
            assert_eq!(stdout(&output), "");
        }
        assert_eq!(sandbox.server.requests().len(), 2);
    }
}

#[test]
fn a_rejected_key_stops_the_run() {
    for status in [401, 403] {
        let sandbox = Sandbox::new(FakeServer::start(move |_| Reply::failure(status)));
        let output = sandbox.run(&["game", WITH_ACHIEVEMENTS, NONE_PUBLISHED]);

        assert_eq!(exit_code(&output), 2, "{status}");
        let stderr = stderr(&output);
        assert!(stderr.starts_with("error: game 1: "), "{stderr}");
        assert!(stderr.contains("RA_API_KEY"), "{stderr}");
        assert_eq!(requested_ids(&sandbox), [WITH_ACHIEVEMENTS], "{status}");
    }
}

#[test]
fn a_rejected_update_gives_the_cached_answer_and_stops_the_run() {
    let sandbox = sandbox_changing_to(|_| Reply::failure(401));
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY);

    let output = sandbox.run(&["game", NONE_PUBLISHED, WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    let stderr = stderr(&output);
    assert!(stderr.starts_with("warning: game 2: "), "{stderr}");
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    assert_eq!(requested_ids(&sandbox), [NONE_PUBLISHED, NONE_PUBLISHED]);
}

#[test]
fn a_failed_update_keeps_the_entry_and_its_age_and_the_run_continues() {
    let sandbox = sandbox_changing_to(|request| match requested_id(request) {
        NONE_PUBLISHED => Reply::failure(500),
        _ => known_games(request),
    });
    cache_game_aged(&sandbox, NONE_PUBLISHED, DAY);

    let output = sandbox.run(&["game", NONE_PUBLISHED, WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    let stderr = stderr(&output);
    assert!(stderr.starts_with("warning: game 2: "), "{stderr}");
    assert!(stderr.contains("No Cheevos: 0 achievements"), "{stderr}");
    assert_eq!(sandbox.server.requests().len(), 3);

    // Still due, so the next run tries again.
    sandbox.run(&["game", NONE_PUBLISHED]);
    assert_eq!(sandbox.server.requests().len(), 4);
}

#[test]
fn a_429_is_retried() {
    let limited = AtomicBool::new(true);
    let sandbox = Sandbox::new(FakeServer::start(move |request| {
        if limited.swap(false, Ordering::Relaxed) {
            return Reply::rate_limited("0");
        }
        known_games(request)
    }));

    let output = sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    assert_eq!(exit_code(&output), 0, "stderr: {}", stderr(&output));
    assert_eq!(sandbox.server.requests().len(), 2);
}

#[test]
fn a_missing_key_says_where_to_get_one_and_exits_2() {
    let sandbox = known_games_sandbox();
    let output = sandbox
        .command(&["game", WITH_ACHIEVEMENTS])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 2);
    assert!(
        stderr(&output).contains("https://retroachievements.org/settings"),
        "stderr: {}",
        stderr(&output)
    );
    assert!(sandbox.server.requests().is_empty());
}

#[test]
fn a_missing_key_stops_the_run_at_the_first_download() {
    let sandbox = known_games_sandbox();
    sandbox.run(&["game", WITH_ACHIEVEMENTS]);

    let output = sandbox
        .command(&["game", WITH_ACHIEVEMENTS, NONE_PUBLISHED, UNKNOWN])
        .env_remove("RA_API_KEY")
        .output()
        .expect("run hascheevos");

    assert_eq!(exit_code(&output), 2, "stderr: {}", stderr(&output));
    // The cached game before it is still answered.
    assert_eq!(stdout(&output).trim(), WITH_ACHIEVEMENTS);
    let stderr = stderr(&output);
    assert_eq!(
        stderr.matches("RA_API_KEY is not set").count(),
        1,
        "{stderr}"
    );
    assert_eq!(sandbox.server.requests().len(), 1);
}

#[test]
fn the_hidden_subcommand_stays_out_of_the_help() {
    let output = known_games_sandbox().run(&["--help"]);
    assert!(!stdout(&output).contains("__fetch-game"));
}
