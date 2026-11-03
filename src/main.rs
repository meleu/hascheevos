mod console;
mod input;
mod rc_hash;

use clap::{Parser, Subcommand};
use std::io::{self, Write};
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
        "unknown console; see `hascheevos hash --help` for the list of names".to_owned()
    })
}

fn main() -> ExitCode {
    let Commands::Hash {
        console,
        verbose,
        file,
    } = Cli::parse().command;

    let input = match input::prepare(&file, console) {
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
