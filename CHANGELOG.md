# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-02-23

### Added
- `--json` flag for NDJSON machine-readable output
- `--version` flag
- Exit code 3 for partial failures (network errors)
- `CheckError::Internal` variant for honest thread-panic reporting
- `clippy::pedantic` lint level
- CLAUDE.md project guide
- CONTRIBUTING.md
- GitHub issue and PR templates
- Pinned toolchain via `rust-toolchain.toml`

### Changed
- Exit code 1 now only means "unavailable" (not "error"); network errors use exit 3
- Thread panics no longer masquerade as `IndexLookup` errors

### Fixed
- Network errors no longer conflated with "name unavailable" in exit code

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
