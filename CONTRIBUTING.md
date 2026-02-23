# Contributing

## Prerequisites

- Rust toolchain (pinned in `rust-toolchain.toml`)

## Getting Started

```sh
git clone https://github.com/rocketman-code/cargo-avail.git
cd cargo-avail
cargo test
```

## Development

```sh
cargo test                     # run all tests
cargo test -- --ignored        # network tests (require internet)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Project Structure

```
src/
  lib.rs       # crate root, lint config
  check.rs     # core checking logic (validation, reserved names, crates.io API)
  main.rs      # CLI, parallel checking, output formatting
tests/
  api.rs       # public library API tests
  cli.rs       # CLI integration tests
```

## Commits

Use conventional commits: `feat(scope): description`, `fix(scope): description`.
