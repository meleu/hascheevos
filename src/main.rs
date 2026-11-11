//! `hc`: tells whether a ROM has RetroAchievements achievements.

mod hash;

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
    #[command(arg_required_else_help = true)]
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
    for console_hash in console_hashes {
        if args.console.is_some() {
            println!("{}", console_hash.hash);
        } else {
            println!(
                "{}\t{}\t{}",
                console_hash.hash,
                console_hash.console,
                console_hash.console_name()
            );
        }
    }
    Ok(())
}
