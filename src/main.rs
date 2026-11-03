mod api;
mod cache;
mod console;
mod input;
mod rc_hash;

use api::FetchError;
use cache::{Entries, Entry, Source};
use clap::{Parser, Subcommand};
use rc_hash::{ConsoleHash, Hashes};
use serde_json::Value;
use std::fmt::Display;
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

    /// Say which ROMs have achievements
    #[command(after_help = format!("Consoles:\n{}", console::help_list()))]
    Check {
        /// Console name or ID; tries each console the file extension could
        /// belong to if omitted
        #[arg(long, value_name = "NAME|ID", value_parser = parse_console)]
        console: Option<u32>,

        /// Download each console's list even if the cached one would do
        #[arg(long)]
        refresh: bool,

        #[arg(required = true, value_name = "FILE")]
        files: Vec<PathBuf>,
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

/// Extensions rcheevos maps to one console on behalf of several others, so a
/// ROM with no achievements may just belong to one of the others.
const SHARED_EXTENSIONS: [&str; 4] = ["bin", "rom", "tap", "nds"];

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
        Commands::Check {
            console,
            refresh,
            files,
        } => check_roms(&files, console, refresh),
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
        warn_update_failed(error);
    }
    Ok((entry, update_result))
}

fn warn_update_failed(error: &FetchError) {
    eprintln!("warning: {error}; this answer may be out of date");
}

fn check_md5(console: u32, refresh: bool, md5: &str) -> ExitCode {
    let md5 = md5.to_ascii_lowercase();
    let loaded = load_current(
        &mut Entries::default(),
        Source::Console(console),
        refresh,
        |list| list.data.get(&md5).is_some_and(earns_achievements),
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

/// Whether a hash from a console's list has achievements to earn.
fn earns_achievements(game: &Value) -> bool {
    achievements(game) > 0 && exclusion_reason(game).is_none()
}

/// Why a hash can't earn its game's achievements, if it's marked that way.
fn exclusion_reason(game: &Value) -> Option<&'static str> {
    match game["compatibility"].as_str()? {
        "incompatible" => Some("marked as incompatible"),
        "patch-required" => Some("the game needs a patch applied to the original ROM"),
        _ => None,
    }
}

fn warn_excluded(item: impl Display, reason: &str) {
    eprintln!("warning: {item}: {reason}; not counted as having achievements");
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
    if let Some(reason) = exclusion_reason(game) {
        warn_excluded(md5, reason);
        return ExitCode::FAILURE;
    }
    match print_result(md5.as_bytes()) {
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
                    if let Err(error) = print_result(id.to_string().as_bytes()) {
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
            // Every other download would fail the same way.
            Some(FetchError::Rejected(_)) => return ExitCode::from(EXIT_ERROR),
            Some(FetchError::Failed(_)) => any_failed = true,
            None => {}
        }
    }
    final_exit_code(any_achievements, any_failed)
}

/// What came of checking one ROM.
#[derive(Default)]
struct RomResult {
    has_achievements: bool,
    /// The hash was found on its console, but can't earn the achievements.
    excluded: bool,
    failed: bool,
    /// Every other download would fail the same way.
    stop_run: bool,
}

impl RomResult {
    fn record_failed_update(&mut self, error: &FetchError) {
        warn_update_failed(error);
        match error {
            FetchError::Rejected(_) => self.stop_run = true,
            FetchError::Failed(_) => self.failed = true,
        }
    }
}

/// A list consulted for a ROM, and the ROM's hash for that list's console.
struct Consulted {
    list: Entry,
    hash: String,
}

impl Consulted {
    fn game(&self) -> Option<&Value> {
        self.list.data.get(&self.hash)
    }

    fn has_achievements(&self) -> bool {
        self.game().is_some_and(earns_achievements)
    }
}

fn check_roms(files: &[PathBuf], console: Option<u32>, refresh: bool) -> ExitCode {
    let mut entries = Entries::default();
    let mut any_achievements = false;
    let mut any_failed = false;
    for file in files {
        let result = check_rom(file, console, refresh, &mut entries);
        any_achievements |= result.has_achievements;
        any_failed |= result.failed;
        if result.stop_run {
            return ExitCode::from(EXIT_ERROR);
        }
    }
    final_exit_code(any_achievements, any_failed)
}

fn check_rom(
    file: &Path,
    named_console: Option<u32>,
    refresh: bool,
    entries: &mut Entries,
) -> RomResult {
    let name = file.display();
    let mut result = RomResult::default();
    let input = match input::prepare(file, named_console) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error: {name}: {error}");
            result.failed = true;
            return result;
        }
    };

    // Search first: a miss on one candidate shouldn't download its list when
    // another candidate has the game.
    let mut hashes = Hashes::new(&input.path, named_console, false);
    let mut any_hash = false;
    let mut consulted = Vec::new();
    while let Some(ConsoleHash { console, hash }) = hashes.next() {
        any_hash = true;
        // After a fallback, the console is only the one rcheevos borrowed.
        if hashes.used_fallback() {
            eprintln!("note: {name}: unrecognised file extension; skipped");
            return result;
        }
        let mut list = match entries.load(Source::Console(console)) {
            Ok(list) => list,
            Err(FetchError::Rejected(message)) => {
                eprintln!("error: {message}");
                result.stop_run = true;
                return result;
            }
            Err(error) => {
                eprintln!("error: {name}: {error}");
                result.failed = true;
                continue;
            }
        };
        if refresh && let Err(error) = entries.update(&mut list) {
            result.record_failed_update(&error);
        }
        let item = Consulted { list, hash };
        let found = item.game().is_some();
        consulted.push(item);
        if found || result.stop_run {
            break;
        }
    }
    if !any_hash {
        for error in hashes.errors() {
            eprintln!("error: {error}");
        }
        eprintln!("error: {name}: no hash produced");
        result.failed = true;
        return result;
    }
    if consulted.is_empty() {
        return result;
    }

    // The refresh rule, for a search that found no achievements. `--refresh` has
    // already updated every list consulted.
    if !refresh && !result.stop_run && !consulted.iter().any(Consulted::has_achievements) {
        for item in &mut consulted {
            if let Err(error) = entries.update_if_due(&mut item.list) {
                result.record_failed_update(&error);
                if result.stop_run {
                    break;
                }
            }
            if item.has_achievements() {
                break;
            }
        }
    }

    report_rom(file, &consulted, &mut result);
    if !result.has_achievements
        && !result.excluded
        && named_console.is_none()
        && SHARED_EXTENSIONS
            .iter()
            .any(|extension| input::has_extension(&input.path, extension))
    {
        eprintln!(
            "note: {name}: this extension can belong to several consoles and only the \
             first was checked; name another with --console"
        );
    }
    result
}

fn report_rom(file: &Path, consulted: &[Consulted], result: &mut RomResult) {
    let name = file.display();
    let matched = consulted
        .iter()
        .find(|item| item.has_achievements())
        .or_else(|| consulted.iter().find(|item| item.game().is_some()));
    let Some(game) = matched.and_then(Consulted::game) else {
        eprintln!("{name}: hash not found");
        return;
    };
    let title = game["title"].as_str().unwrap_or_default();
    let achievements = achievements(game);
    eprintln!("{name}: {title}: {achievements} achievements");
    if achievements == 0 {
        return;
    }
    if let Some(reason) = exclusion_reason(game) {
        warn_excluded(name, reason);
        result.excluded = true;
    } else {
        result.has_achievements = true;
        if let Err(error) = print_result(file.as_os_str().as_encoded_bytes()) {
            eprintln!("error: {error}");
            result.failed = true;
        }
    }
}

fn final_exit_code(any_achievements: bool, any_failed: bool) -> ExitCode {
    if any_failed {
        ExitCode::from(EXIT_ERROR)
    } else if any_achievements {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_result(answer: &[u8]) -> io::Result<()> {
    let mut stdout = io::stdout();
    match stdout
        .write_all(answer)
        .and_then(|()| stdout.write_all(b"\n"))
    {
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
