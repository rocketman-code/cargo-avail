# cargo-avail

[![CI](https://github.com/rocketman-code/cargo-avail/actions/workflows/ci.yml/badge.svg)](https://github.com/rocketman-code/cargo-avail/actions/workflows/ci.yml)

Check whether crate names are truly available on crates.io.

Unlike simple "does this crate exist?" checkers, `cargo-avail` uses the
actual crates.io validation logic to catch names that would be rejected at
publish time:

- Syntactic validation (character rules, length limits, leading digits)
- Reserved names (Rust internals like `std`, `core`, `alloc`; Windows device
  names like `nul`, `con`, `aux`, `com0`-`com9`, `lpt0`-`lpt9`)
- Canonical collision detection (hyphens and underscores are equivalent:
  `foo-bar` and `foo_bar` are the same crate)
- Sparse index lookup against `index.crates.io`

## Install

```sh
cargo install cargo-avail
```

## Usage

```sh
# Check one or more names
cargo avail my-crate another-name

# Pipe names from stdin
echo "my-crate\nanother-name" | cargo avail

# Only show available names
cargo avail --available-only name1 name2 name3

# Quiet mode: exit code only (0 = all available, 1 = any unavailable)
cargo avail --quiet my-crate

# JSON output for scripting
cargo avail --json my-crate another-name | jq '.status'

# Check version
cargo avail --version
```

## Output

Tab-separated: `name\tstatus`

```
my-crate        available
serde           taken
std             reserved
foo+bar         invalid: invalid character `+` in crate name: `foo+bar`, characters must be ASCII alphanumeric, `-`, or `_`
```

With `--json`, one JSON object per line (NDJSON):

```json
{"name":"my-crate","status":"available"}
{"name":"serde","status":"taken"}
{"name":"std","status":"reserved"}
{"name":"foo+bar","status":"invalid","error":"invalid character `+` in crate name: `foo+bar`, characters must be ASCII alphanumeric, `-`, or `_`"}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | All names are available |
| 1    | One or more names are unavailable (taken, reserved, or invalid) |
| 2    | Usage error (no names provided, stdin read failure) |
| 3    | Partial failure: some names could not be checked (network error) |

## Library Usage

`cargo-avail` also exposes a library crate for programmatic use:

```rust,no_run
use cargo_avail::check::{Client, check_name, Availability};

let client = Client::new();
match check_name(&client, "my-cool-crate") {
    Ok(Availability::Available) => println!("go grab it!"),
    Ok(status) => println!("{status}"),
    Err(e) => eprintln!("error: {e}"),
    _ => {} // non_exhaustive
}
```

## Limitations

- Cannot detect recently deleted crates (requires database access).
- A name passing all checks could still fail at publish time due to
  server-side race conditions or policy changes.
- Mixed-separator names (e.g., `my_cool-crate`) are checked via the three
  most common variants, not all 2^n permutations.

## License

MIT
