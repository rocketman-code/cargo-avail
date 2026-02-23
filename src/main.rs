use std::fmt::Write as _;
use std::io::{self, BufRead, IsTerminal};
use std::process::ExitCode;

use clap::Parser;
use serde::Serialize;

use cargo_avail::check::{
    Availability, CheckError, Client, MAX_CONCURRENT_REQUESTS, canon_crate_name, check_name,
};

#[derive(Serialize)]
struct JsonResult {
    name: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Parser)]
#[command(
    name = "cargo-avail",
    bin_name = "cargo avail",
    version,
    about = "Check whether crate names are truly available on crates.io",
    after_help = "Checks name validity (character rules, length), reserved names \
                  (std, core, alloc, nul, com0, etc.), and the crates.io sparse index \
                  with canonical matching (hyphens and underscores are equivalent).\n\n\
                  Cannot detect recently deleted crates (requires DB access). \
                  A name passing all checks could still fail at publish time.",
    allow_hyphen_values = true
)]
struct Cli {
    /// Crate names to check (also reads from stdin)
    names: Vec<String>,

    /// Suppress output, exit code only
    #[arg(short, long, conflicts_with = "json")]
    quiet: bool,

    /// Only print available names
    #[arg(short, long, conflicts_with = "json")]
    available_only: bool,

    /// Output results as NDJSON (one JSON object per line)
    #[arg(long)]
    json: bool,
}

/// Sanitize a string for tab-separated output: replace control chars with escape sequences.
fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c if c.is_control() => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so piping to head/grep/etc. does not panic.
    // On Unix, the default disposition for SIGPIPE is to terminate the process.
    // Rust's runtime sets SIG_IGN for SIGPIPE, which causes write() to return
    // EPIPE and println!() to panic. Restoring default behavior lets the OS
    // handle it cleanly.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    // Strip the subcommand name when invoked as `cargo avail`.
    // Detect this by checking if argv[0] basename is "cargo" and argv[1] is "avail".
    // When invoked directly as `cargo-avail avail`, argv[0] is "cargo-avail",
    // so "avail" is a real crate name and should NOT be stripped.
    let args: Vec<String> = std::env::args().collect();
    let invoked_via_cargo = args.first().is_some_and(|a| {
        let base = a.rsplit('/').next().unwrap_or(a);
        base == "cargo"
    });
    let args = if invoked_via_cargo && args.len() > 1 && args[1] == "avail" {
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
        eprintln!("usage: cargo avail [OPTIONS] [NAMES...]");
        return ExitCode::from(2);
    }

    // Deduplicate by canonical name while preserving order and original input
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(canon_crate_name(n)));

    let client = Client::new();

    // Check names in parallel, capped at MAX_CONCURRENT_REQUESTS threads
    let mut results: Vec<(String, Result<Availability, CheckError>)> =
        Vec::with_capacity(names.len());
    for chunk in names.chunks(MAX_CONCURRENT_REQUESTS) {
        let chunk_results: Vec<_> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|name| {
                    let client = &client;
                    s.spawn(move || (name.clone(), check_name(client, name)))
                })
                .collect();
            handles
                .into_iter()
                .zip(chunk)
                .map(|(h, original_name)| {
                    h.join().unwrap_or_else(|_| {
                        (
                            original_name.clone(),
                            Err(CheckError::Internal("thread panic".into())),
                        )
                    })
                })
                .collect()
        });
        results.extend(chunk_results);
    }

    let mut any_unavailable = false;
    let mut error_count: usize = 0;

    for (name, result) in &results {
        let is_available = matches!(result, Ok(Availability::Available));
        // Network/internal errors mean we couldn't determine availability.
        // InvalidName is deterministic -- the name is definitively unavailable.
        let is_network_error = matches!(
            result,
            Err(CheckError::IndexLookup(_) | CheckError::Internal(_))
        );

        if !is_available && !is_network_error {
            any_unavailable = true;
        }

        if is_network_error {
            error_count += 1;
        }

        if cli.quiet {
            continue;
        }

        if cli.json {
            let json_result = match result {
                Ok(a) => JsonResult {
                    name: name.clone(),
                    status: a.to_string(),
                    error: None,
                },
                Err(CheckError::InvalidName(e)) => JsonResult {
                    name: name.clone(),
                    status: "invalid".to_string(),
                    error: Some(e.to_string()),
                },
                Err(e) => JsonResult {
                    name: name.clone(),
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                },
            };
            println!(
                "{}",
                serde_json::to_string(&json_result).expect("JSON serialization should not fail")
            );
            continue;
        }

        // --available-only hides taken/reserved/invalid but always shows errors
        if cli.available_only && !is_available && !is_network_error {
            continue;
        }

        let status_str = match result {
            Ok(a) => a.to_string(),
            Err(e) => e.to_string(),
        };
        let sanitized_name = sanitize(name);
        let sanitized_status = sanitize(&status_str);
        println!("{sanitized_name}\t{sanitized_status}");
    }

    if error_count > 0 && !cli.quiet {
        eprintln!(
            "warning: {error_count} name{} could not be checked (network error)",
            if error_count == 1 { "" } else { "s" }
        );
    }

    if error_count > 0 {
        ExitCode::from(3)
    } else if any_unavailable {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
