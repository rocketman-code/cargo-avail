# cargo-check-name Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a cargo subcommand that checks crate name availability using real crates.io validation logic.

**Architecture:** Git-depend on `crates_io_validation` from `rust-lang/crates.io` for name validation. Implement canonicalization, reserved name checking, and sparse index lookups locally. Single binary, sync HTTP, no async runtime.

**Tech Stack:** Rust, clap (CLI), ureq (HTTP), crates_io_validation (git dep from rust-lang/crates.io)

---

### Task 1: Scaffold the project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

**Step 1: Initialize the cargo project**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo init --name cargo-check-name`

**Step 2: Set up Cargo.toml with dependencies**

`Cargo.toml` should contain:

```toml
[package]
name = "cargo-check-name"
version = "0.1.0"
edition = "2024"
description = "Check whether crate names are truly available on crates.io"
license = "MIT"

[[bin]]
name = "cargo-check-name"
path = "src/main.rs"

[dependencies]
crates_io_validation = { git = "https://github.com/rust-lang/crates.io", default-features = false }
clap = { version = "4", features = ["derive"] }
ureq = "3"
```

**Step 3: Write a minimal main.rs that compiles**

```rust
fn main() {
    println!("cargo-check-name");
}
```

**Step 4: Verify it compiles**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo build`
Expected: Compiles successfully, fetches git dep from rust-lang/crates.io

**Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs
git commit -m "chore: scaffold cargo-check-name project"
```

---

### Task 2: Implement core checking logic

**Files:**
- Create: `src/check.rs`
- Create: `src/lib.rs`
- Test: inline `#[cfg(test)]` in `src/check.rs`

**Step 1: Write failing tests for check logic**

Create `src/check.rs` with tests only:

```rust
use std::collections::HashSet;

const RESERVED_NAMES: &[&str] = &[
    "alloc", "arena", "ast", "builtins", "collections", "compiler-builtins",
    "compiler-rt", "compiletest", "core", "coretest", "debug", "driver",
    "flate", "fmt_macros", "grammar", "graphviz", "macro", "macros",
    "proc_macro", "rbml", "rust-installer", "rustbook", "rustc", "rustc_back",
    "rustc_borrowck", "rustc_driver", "rustc_llvm", "rustc_resolve",
    "rustc_trans", "rustc_typeck", "rustdoc", "rustllvm", "rustuv",
    "serialize", "std", "syntax", "test", "unicode",
];

pub enum Status {
    Available,
    Taken,
    Reserved,
    Invalid(String),
    Unknown(String),
}

pub fn canon_crate_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

fn index_path(canonical: &str) -> String {
    match canonical.len() {
        0 => unreachable!("empty name should be caught by validation"),
        1 => format!("1/{canonical}"),
        2 => format!("2/{canonical}"),
        3 => format!("3/{}/{canonical}", &canonical[..1]),
        _ => format!("{}/{}/{canonical}", &canonical[..2], &canonical[2..4]),
    }
}

fn reserved_set() -> HashSet<String> {
    RESERVED_NAMES.iter().map(|s| canon_crate_name(s)).collect()
}

pub fn check_name(name: &str) -> Status {
    // 1. Validate
    if let Err(e) = crates_io_validation::validate_crate_name("crate", name) {
        return Status::Invalid(e.to_string());
    }

    let canonical = canon_crate_name(name);

    // 2. Reserved
    if reserved_set().contains(&canonical) {
        return Status::Reserved;
    }

    // 3. Sparse index -- try both hyphen and underscore variants
    let hyphen_variant = canonical.replace('_', "-");
    let underscore_variant = &canonical;

    let variants: Vec<&str> = if hyphen_variant == *underscore_variant {
        vec![underscore_variant]
    } else {
        vec![underscore_variant, &hyphen_variant]
    };

    for variant in variants {
        let path = index_path(variant);
        let url = format!("https://index.crates.io/{path}");
        match ureq::get(&url).call() {
            Ok(resp) if resp.status() == 200 => return Status::Taken,
            Ok(_) | Err(ureq::Error::StatusCode(404)) => continue,
            Err(e) => return Status::Unknown(e.to_string()),
        }
    }

    Status::Available
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canon_crate_name() {
        assert_eq!(canon_crate_name("My-Crate"), "my_crate");
        assert_eq!(canon_crate_name("foo_bar"), "foo_bar");
        assert_eq!(canon_crate_name("FOO"), "foo");
    }

    #[test]
    fn test_index_path_1_char() {
        assert_eq!(index_path("a"), "1/a");
    }

    #[test]
    fn test_index_path_2_char() {
        assert_eq!(index_path("ab"), "2/ab");
    }

    #[test]
    fn test_index_path_3_char() {
        assert_eq!(index_path("abc"), "3/a/abc");
    }

    #[test]
    fn test_index_path_4_plus_char() {
        assert_eq!(index_path("serde"), "se/rd/serde");
        assert_eq!(index_path("abcd"), "ab/cd/abcd");
    }

    #[test]
    fn test_reserved_set_contains_canonicalized() {
        let set = reserved_set();
        assert!(set.contains("std"));
        assert!(set.contains("compiler_builtins")); // hyphen -> underscore
        assert!(set.contains("rust_installer"));
    }

    #[test]
    fn test_invalid_name() {
        match check_name("foo+bar") {
            Status::Invalid(msg) => assert!(msg.contains("invalid character")),
            _ => panic!("expected Invalid"),
        }
    }

    #[test]
    fn test_reserved_name() {
        match check_name("std") {
            Status::Reserved => {}
            _ => panic!("expected Reserved"),
        }
    }

    #[test]
    fn test_reserved_name_canonical_match() {
        match check_name("Compiler-Builtins") {
            Status::Reserved => {}
            _ => panic!("expected Reserved for canonical match"),
        }
    }

    #[test]
    fn test_taken_name() {
        // serde definitely exists
        match check_name("serde") {
            Status::Taken => {}
            _ => panic!("expected Taken"),
        }
    }

    #[test]
    fn test_available_name() {
        // extremely unlikely to be taken
        match check_name("zzzyyyxxxwww-not-a-real-crate") {
            Status::Available => {}
            _ => panic!("expected Available"),
        }
    }

    #[test]
    fn test_canonical_collision_detected() {
        // tokio-util exists, so tokio_util should be taken
        match check_name("tokio_util") {
            Status::Taken => {}
            _ => panic!("expected Taken for canonical collision"),
        }
    }
}
```

Create `src/lib.rs`:

```rust
pub mod check;
```

**Step 2: Run tests to verify they fail (compile error since check_name isn't wired up)**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo test`
Expected: Tests compile and some pass (unit tests for canon/index_path), integration tests pass for validation/reserved/taken/available

Note: These tests should actually all pass since the implementation is inline with the tests. The TDD cycle here is: write tests + implementation together for the core module, then verify.

**Step 3: Run tests to verify they pass**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add src/lib.rs src/check.rs
git commit -m "feat(check): implement core name availability checking"
```

---

### Task 3: Implement CLI

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the CLI with clap**

Replace `src/main.rs`:

```rust
use std::io::{self, BufRead, IsTerminal};
use std::process::ExitCode;

use clap::Parser;

use cargo_check_name::check::{check_name, Status};

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

    let mut all_available = true;

    for name in &names {
        let status = check_name(name);
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
```

**Step 2: Build and test manually**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo build && cargo run -- serde std foo+bar zzzyyyxxxwww-not-a-real-crate`
Expected output (tab-separated):
```
serde	taken
std	reserved
foo+bar	invalid: ...
zzzyyyxxxwww-not-a-real-crate	available
```
Expected exit code: 1

**Step 3: Test quiet mode**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo run -- -q serde; echo $?`
Expected: no output, exit code 1

**Step 4: Test available-only mode**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo run -- -a serde zzzyyyxxxwww-not-a-real-crate`
Expected: only prints the available name

**Step 5: Test stdin piping**

Run: `echo -e "serde\nstd" | cd ~/dev/rocketman-code/crate-checker && cargo run`
Expected: shows both as taken/reserved

**Step 6: Test cargo subcommand invocation**

Run: `cd ~/dev/rocketman-code/crate-checker && cargo install --path . && cargo check-name serde std`
Expected: works as cargo subcommand

**Step 7: Commit**

```bash
git add src/main.rs
git commit -m "feat(cli): implement cargo check-name CLI interface"
```

---

### Task 4: Run the original user command and verify

**Step 1: Run the exact command from the original request**

Run: `cargo check-name bassoon saxophone piano drums banjo harmonica ukulele trumpet trombone clarinet harp`

Expected: each instrument name checked and reported as available/taken with exit code.

**Step 2: Commit any final tweaks**

---

### Task 5: Clean up and finalize

**Step 1: Add .gitignore**

Create `.gitignore`:

```
/target
```

**Step 2: Final commit**

```bash
git add .gitignore
git commit -m "chore: add gitignore"
```
