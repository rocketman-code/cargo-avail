# cargo-check-name Design

A cargo subcommand that checks whether crate names are truly available on crates.io, using the real crates.io validation logic monkey-patched via git dependency.

## Problem

Existing tools (e.g. `checker`, `cargo-name`) only hit the crates.io REST API for a 200/404 check. This misses:
- Reserved names (std, core, alloc, rustc, etc.)
- Canonical name collisions (my-crate == my_crate == My-Crate)
- Name validation rules (character restrictions, length limits)
- Deleted crates (cooldown period after deletion)

## Approach

Git-depend on `crates_io_validation` from the real `rust-lang/crates.io` repo. Replicate the trivial canonicalization logic. Hardcode the reserved names list from crates.io's migration. Check the sparse index for existing crates.

## Checks (in order)

1. Name validation -- `crates_io_validation::validate_crate_name` (git dep)
2. Reserved names -- hardcoded set from crates.io migration (std, core, alloc, rustc, etc.)
3. Sparse index lookup -- HTTP GET to `https://index.crates.io/{prefix}/{name}` with canonicalized name. 200 = taken, 404 = available.

## Canonicalization

`canon_crate_name(name) = name.to_lowercase().replace('-', '_')`

Matches the SQL function in crates.io: `SELECT replace(lower($1), '-', '_')`

## Reserved Names

From crates.io migration `20170305095748_create_reserved_crate_names`:

alloc, arena, ast, builtins, collections, compiler-builtins, compiler-rt,
compiletest, core, coretest, debug, driver, flate, fmt_macros, grammar,
graphviz, macro, macros, proc_macro, rbml, rust-installer, rustbook, rustc,
rustc_back, rustc_borrowck, rustc_driver, rustc_llvm, rustc_resolve,
rustc_trans, rustc_typeck, rustdoc, rustllvm, rustuv, serialize, std,
syntax, test, unicode

## Sparse Index

The crates.io sparse index lives at `https://index.crates.io/`. Path structure:
- 1-char names: `1/{name}`
- 2-char names: `2/{name}`
- 3-char names: `3/{first-char}/{name}`
- 4+ char names: `{first-two}/{next-two}/{name}`

Names are lowercased in the index. We canonicalize before lookup.

## CLI Interface

```
cargo check-name [OPTIONS] [NAMES...]
```

Reads names from args and/or stdin (one per line), deduplicated.

### Output

One line per name, tab-separated:

```
bassoon     available
std         reserved
foo+bar     invalid: invalid character `+` in crate name: `foo+bar`
my-crate    taken
```

### Exit Codes

- 0: all names available
- 1: at least one name unavailable (taken/reserved/invalid)
- 2: usage error or network failure

### Flags

- `-q` / `--quiet`: suppress output, exit code only
- `-a` / `--available-only`: only print available names

## Dependencies

- `crates_io_validation` (git dep from rust-lang/crates.io)
- `clap` (CLI parsing)
- `ureq` (HTTP, sync, no tokio)

## Limitations

Cannot check deleted crates (requires DB access). A name passing all checks could still fail at publish time if it was recently deleted. Noted in --help.
