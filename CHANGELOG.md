# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-02-22

Initial release.

### Added
- Validate crate names using vendored crates.io validation logic
- Check against reserved names (Rust internals, Windows device names)
- Sparse index lookup with canonical name collision detection
- Parallel checking with bounded concurrency (20 concurrent requests)
- Stdin piping support for batch checking
- `--quiet` flag for exit-code-only mode
- `--available-only` flag to filter output
- Tab-separated output format
- Library API (`cargo_avail::check`) for programmatic use
