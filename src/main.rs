//! `hc`: tells whether a ROM has RetroAchievements achievements.

mod hash;

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hc", about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the hash RetroAchievements uses to identify a ROM
    ///
    /// With --console, prints the bare hash. Without, prints one line per
    /// console that produced a hash, in the order rcheevos tried them:
    ///
    ///     <hash><TAB><console id><TAB><console name>
    ///
    /// The console ID on each line is usable as the --console value.
    #[command(arg_required_else_help = true, verbatim_doc_comment)]
    Hash(HashArgs),
}

#[derive(Args)]
struct HashArgs {
    /// Numeric console ID whose official method computes the hash
    #[arg(long, value_name = "ID")]
    console: Option<u32>,

    /// The ROM to hash
    rom: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome = match cli.command {
        Command::Hash(args) => print_hashes(&args),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("hc: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn print_hashes(args: &HashArgs) -> Result<()> {
    let console_hashes = hash::console_hashes(&args.rom, args.console)?;
    if args.console.is_some() {
        // A bare hash, so scripts can capture it with `$(...)`.
        for console_hash in &console_hashes {
            println!("{}", console_hash.hash);
        }
        return Ok(());
    }

    // The header is for a person reading the table, so scripts that capture
    // stdout never see it; it goes to stderr to keep stdout pure data.
    if io::stdout().is_terminal() {
        eprintln!("HASH\tCONSOLE ID\tCONSOLE NAME");
    }
    for console_hash in &console_hashes {
        println!(
            "{}\t{}\t{}",
            console_hash.hash,
            console_hash.console,
            console_hash.console_name()
        );
    }
    Ok(())
}
