mod api;
mod cache;
mod console;
mod input;
mod rc_hash;

use clap::{Parser, Subcommand};
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

        md5: String,
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
        Commands::Md5 { console, md5 } => check_md5(console, &md5),
        Commands::FetchSystem {
            format: _,
            system_id,
        } => fetch_system(system_id),
    }
}

fn check_md5(console: u32, md5: &str) -> ExitCode {
    let list = match cache::system_list(console) {
        Ok(list) => list,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(EXIT_ERROR);
        }
    };

    let md5 = md5.to_ascii_lowercase();
    let Some(game) = list.get(&md5) else {
        eprintln!("{md5}: hash not found");
        return ExitCode::FAILURE;
    };
    let title = game["title"].as_str().unwrap_or_default();
    let achievements = game["achievements"].as_u64().unwrap_or(0);
    eprintln!("{md5}: {title}: {achievements} achievements");
    if achievements == 0 {
        return ExitCode::FAILURE;
    }
    match writeln!(io::stdout(), "{md5}") {
        // Whoever was reading closed the pipe, which doesn't change the answer.
        Err(error) if error.kind() != io::ErrorKind::BrokenPipe => {
            eprintln!("error: {error}");
            ExitCode::from(EXIT_ERROR)
        }
        _ => ExitCode::SUCCESS,
    }
}

fn fetch_system(system_id: u32) -> ExitCode {
    match api::fetch_system(system_id) {
        Ok(list) => {
            print!("{list}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(EXIT_ERROR)
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
