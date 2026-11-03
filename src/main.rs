mod api;
mod cache;
mod console;
mod input;
mod rc_hash;

use api::FetchError;
use cache::{Entries, Entry, Source};
use clap::{Parser, Subcommand};
use serde_json::Value;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the RetroAchievements hashes of a ROM or disc file
    #[command(after_help = format!("Consoles:\n{}", console::help_list()))]
    Hash {
        /// Console name or ID; tries each console the file extension could
        /// belong to if omitted
        #[arg(long, value_name = "NAME|ID", value_parser = parse_console)]
        console: Option<u32>,

        /// Report rcheevos progress on stderr
        #[arg(short, long)]
        verbose: bool,

        file: PathBuf,
    },

    /// Say whether an MD5 hash has achievements
    #[command(after_help = format!("Consoles:\n{}", console::help_list()))]
    Md5 {
        /// Console name or ID; required because the API can't search by hash
        #[arg(long, value_name = "NAME|ID", value_parser = parse_console)]
        console: u32,

        /// Download the console's list even if the cached one would do
        #[arg(long)]
        refresh: bool,

        md5: String,
    },

    /// Say whether games have achievements
    Game {
        /// Download the games even if the cached ones would do
        #[arg(long)]
        refresh: bool,

        #[arg(required = true, value_name = "ID")]
        ids: Vec<u32>,
    },

    /// Print a console's list as compact JSON, for the cache to store
    #[command(name = "__fetch-system", hide = true)]
    FetchSystem {
        /// Version of the printed format, so that changing it changes the cache
        /// key
        #[arg(long)]
        format: u32,

        system_id: u32,
    },

    /// Print a game as compact JSON, for the cache to store
    #[command(name = "__fetch-game", hide = true)]
    FetchGame {
        /// Version of the printed format, so that changing it changes the cache
        /// key
        #[arg(long)]
        format: u32,

        game_id: u32,
    },
}

/// Exit status for "something failed", as distinct from "no achievements".
const EXIT_ERROR: u8 = 2;

fn parse_console(value: &str) -> Result<u32, String> {
    console::parse(value)
        .ok_or_else(|| "unknown console; see `--help` for the list of names".to_owned())
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Commands::Hash {
            console,
            verbose,
            file,
        } => hash(console, verbose, &file),
        Commands::Md5 {
            console,
            refresh,
            md5,
        } => check_md5(console, refresh, &md5),
        Commands::Game { refresh, ids } => check_games(&ids, refresh),
        Commands::FetchSystem {
            format: _,
            system_id,
        } => print_fetched(api::fetch_system(system_id)),
        Commands::FetchGame { format: _, game_id } => print_fetched(api::fetch_game(game_id)),
    }
}

/// Returns a source's entry, updated if `--refresh` or the refresh rule call
/// for it. A failed update comes back beside the entry, whose answer stands.
fn load_current(
    entries: &mut Entries,
    source: Source,
    refresh: bool,
    has_achievements: impl Fn(&Entry) -> bool,
) -> Result<(Entry, Result<(), FetchError>), FetchError> {
    let mut entry = entries.load(source)?;
    let mut update_result = if refresh {
        entries.update(&mut entry)
    } else {
        Ok(())
    };
    if update_result.is_ok() && !has_achievements(&entry) {
        update_result = entries.update_if_due(&mut entry);
    }
    if let Err(error) = &update_result {
        eprintln!("warning: {error}; this answer may be out of date");
    }
    Ok((entry, update_result))
}

fn check_md5(console: u32, refresh: bool, md5: &str) -> ExitCode {
    let md5 = md5.to_ascii_lowercase();
    let loaded = load_current(
        &mut Entries::default(),
        Source::Console(console),
        refresh,
        |list| {
            list.data
                .get(&md5)
                .is_some_and(|game| achievements(game) > 0)
        },
    );
    let (list, update_result) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    let answer = report_md5(&list, &md5);
    if update_result.is_err() {
        return ExitCode::from(EXIT_ERROR);
    }
    answer
}

fn achievements(game: &Value) -> u64 {
    game["achievements"].as_u64().unwrap_or(0)
}

fn report_md5(list: &Entry, md5: &str) -> ExitCode {
    let Some(game) = list.data.get(md5) else {
        eprintln!("{md5}: hash not found");
        return ExitCode::FAILURE;
    };
    let title = game["title"].as_str().unwrap_or_default();
    let achievements = achievements(game);
    eprintln!("{md5}: {title}: {achievements} achievements");
    if achievements == 0 {
        return ExitCode::FAILURE;
    }
    match print_result(md5) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(EXIT_ERROR)
        }
    }
}

fn check_games(ids: &[u32], refresh: bool) -> ExitCode {
    let mut entries = Entries::default();
    let mut any_achievements = false;
    let mut any_failed = false;
    for id in ids {
        let source = Source::Game(*id);
        let loaded = load_current(&mut entries, source, refresh, |game| {
            achievements(&game.data) > 0
        });
        let failure = match loaded {
            Ok((game, update_result)) => {
                let title = game.data["title"].as_str().unwrap_or_default();
                let achievements = achievements(&game.data);
                eprintln!("{source}: {title}: {achievements} achievements");
                if achievements > 0 {
                    any_achievements = true;
                    if let Err(error) = print_result(&id.to_string()) {
                        eprintln!("error: {error}");
                        any_failed = true;
                    }
                }
                update_result.err()
            }
            Err(error) => {
                eprintln!("error: {error}");
                Some(error)
            }
        };
        match failure {
            // Every other game would be rejected the same way.
            Some(FetchError::Rejected(_)) => return ExitCode::from(EXIT_ERROR),
            Some(FetchError::Failed(_)) => any_failed = true,
            None => {}
        }
    }
    if any_failed {
        ExitCode::from(EXIT_ERROR)
    } else if any_achievements {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_result(answer: &str) -> io::Result<()> {
    match writeln!(io::stdout(), "{answer}") {
        // Whoever was reading closed the pipe, which doesn't change the answer.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        written => written,
    }
}

fn print_fetched(fetched: Result<String, FetchError>) -> ExitCode {
    match fetched {
        Ok(entry) => {
            print!("{entry}");
            ExitCode::SUCCESS
        }
        // Stdout, because the parent streams stderr to the user as progress
        // and reports this itself.
        Err(error) => {
            print!("{error}");
            ExitCode::from(match error {
                FetchError::Rejected(_) => api::EXIT_REJECTED,
                FetchError::Failed(_) => EXIT_ERROR,
            })
        }
    }
}

fn hash(console: Option<u32>, verbose: bool, file: &Path) -> ExitCode {
    let input = match input::prepare(file, console) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: {}: {error}", file.display());
            return ExitCode::FAILURE;
        }
    };

    let report = rc_hash::hash_file(&input.path, console, verbose);

    let hashes = report.distinct_hashes();
    for hash in &hashes {
        match writeln!(io::stdout(), "{hash}") {
            Ok(()) => {}
            // Whoever was reading has what they wanted, like `... | head -1`.
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => break,
            Err(error) => {
                eprintln!("error: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    if hashes.is_empty() {
        if !verbose {
            for error in &report.errors {
                eprintln!("error: {error}");
            }
        }
        eprintln!("error: {}: no hash produced", file.display());
        return ExitCode::FAILURE;
    }
    if report.used_fallback() {
        eprintln!("warning: unknown file extension; printed a hash of the whole file");
    }
    ExitCode::SUCCESS
}
