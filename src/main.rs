use std::io::{self, BufRead, IsTerminal};
use std::process::ExitCode;

use clap::Parser;

use cargo_check_name::check::{check_name, new_agent, Status};

#[derive(Parser)]
#[command(
    name = "cargo-check-name",
    bin_name = "cargo check-name",
    about = "Check whether crate names are truly available on crates.io",
    after_help = "Checks name validity (character rules, length), reserved names \
                  (std, core, alloc, etc.), and the crates.io sparse index with \
                  canonical matching (hyphens and underscores are equivalent).\n\n\
                  Cannot detect recently deleted crates (requires DB access). \
                  A name passing all checks could still fail at publish time."
)]
struct Cli {
    /// Crate names to check (also reads from stdin)
    names: Vec<String>,

    /// Suppress output, exit code only
    #[arg(short, long)]
    quiet: bool,

    /// Only print available names
    #[arg(short, long)]
    available_only: bool,
}

fn main() -> ExitCode {
    // cargo passes "check-name" as first arg when invoked as subcommand
    let args: Vec<String> = std::env::args().collect();
    let args = if args.len() > 1 && args[1] == "check-name" {
        [&args[..1], &args[2..]].concat()
    } else {
        args
    };

    let cli = Cli::parse_from(args);

    let mut names: Vec<String> = cli.names;

    // Read from stdin if not a terminal
    if !io::stdin().is_terminal() {
        for line in io::stdin().lock().lines() {
            match line {
                Ok(l) => {
                    let trimmed = l.trim().to_string();
                    if !trimmed.is_empty() {
                        names.push(trimmed);
                    }
                }
                Err(e) => {
                    eprintln!("error: reading stdin: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    }

    if names.is_empty() {
        eprintln!("error: no crate names provided");
        eprintln!("usage: cargo check-name [OPTIONS] [NAMES...]");
        return ExitCode::from(2);
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.clone()));

    let agent = new_agent();
    let mut all_available = true;

    for name in &names {
        let status = check_name(&agent, name);
        let (label, detail) = match &status {
            Status::Available => ("available", String::new()),
            Status::Taken => ("taken", String::new()),
            Status::Reserved => ("reserved", String::new()),
            Status::Invalid(msg) => ("invalid", format!(": {msg}")),
            Status::Unknown(msg) => ("unknown", format!(": {msg}")),
        };

        if !matches!(status, Status::Available) {
            all_available = false;
        }

        if cli.quiet {
            continue;
        }

        if cli.available_only && !matches!(status, Status::Available) {
            continue;
        }

        println!("{name}\t{label}{detail}");
    }

    if all_available {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
