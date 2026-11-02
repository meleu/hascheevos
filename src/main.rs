mod console;
mod rc_hash;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
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
}

fn parse_console(value: &str) -> Result<u32, String> {
    console::parse(value).ok_or_else(|| {
        format!("unknown console; see `hascheevos hash --help` for the list of names")
    })
}

fn main() -> ExitCode {
    let Commands::Hash { console, verbose, file } = Cli::parse().command;

    if !file.exists() {
        eprintln!("error: {}: no such file", file.display());
        return ExitCode::FAILURE;
    }
    if !file.is_file() && console != Some(rc_hash::RC_CONSOLE_ARCADE) {
        eprintln!("error: {}: not a file", file.display());
        return ExitCode::FAILURE;
    }

    let report = rc_hash::hash_file(&file, console, verbose);

    for hash in &report.hashes {
        println!("{hash}");
    }
    if report.hashes.is_empty() {
        if !verbose {
            for error in &report.errors {
                eprintln!("error: {error}");
            }
        }
        eprintln!("error: {}: no hash produced", file.display());
        return ExitCode::FAILURE;
    }
    if report.used_fallback() && !verbose {
        eprintln!("warning: unknown file extension; printed a hash of the whole file");
    }
    ExitCode::SUCCESS
}
