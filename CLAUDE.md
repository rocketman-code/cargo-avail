# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## What This Is

cargo-avail is a Rust CLI and library that checks whether crate names are available on crates.io. Published as `cargo-avail` on crates.io. Invoked as `cargo avail`.

## Commands

```bash
cargo build                    # debug build
cargo test                     # all tests (~0.5s)
cargo test -- --ignored        # network tests (require internet)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Toolchain pinned in `rust-toolchain.toml` (rustc 1.85.0).

## Architecture

Three source files:

1. `src/lib.rs` -- Crate root. `#![warn(clippy::pedantic)]`.
2. `src/check.rs` -- Core logic: vendored crates.io validation, reserved names (56 entries from 3 DB migrations), sparse index HTTP lookups via ureq. Public types: `Availability`, `CheckError`, `Client`. `canon_crate_name()` normalizes hyphens/underscores.
3. `src/main.rs` -- CLI via clap derive. SIGPIPE handling, cargo subcommand detection, parallel checking (thread::scope, 20 max concurrent), tab-separated and NDJSON output.

## Testing

- Unit tests in `src/check.rs` (canonicalization, index paths, reserved names, validation)
- Property tests via proptest (idempotency, invariants, separator equivalence)
- Auto-trait tests (Send + Sync + Unpin on public types)
- Integration tests in `tests/api.rs` (public API) and `tests/cli.rs` (CLI behavior)
- Network tests marked `#[ignore]` (run with `cargo test -- --ignored`)

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | All available |
| 1    | At least one unavailable (taken/reserved/invalid) |
| 2    | Usage error |
| 3    | Partial failure (network errors) |
