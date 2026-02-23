//! Core availability checking logic for crate names on crates.io.

use std::collections::HashSet;
use std::fmt;
use std::sync::LazyLock;
use std::time::Duration;

use ureq::Agent;

// Vendored from rust-lang/crates.io crates_io_validation crate (commit 046368f4).
// Cannot use as a dependency because it's not published on crates.io.
// Source: crates/crates_io_validation/src/lib.rs
pub(crate) mod validation {
    use std::fmt;

    pub const MAX_NAME_LENGTH: usize = 64;

    #[derive(Debug)]
    pub enum InvalidCrateName {
        TooLong { name: String },
        Empty,
        StartWithDigit { name: String },
        Start { first_char: char, name: String },
        Char { ch: char, name: String },
    }

    impl fmt::Display for InvalidCrateName {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::TooLong { name } => {
                    write!(
                        f,
                        "crate name `{name}` is too long (max {MAX_NAME_LENGTH} characters)"
                    )
                }
                Self::Empty => write!(f, "crate name cannot be empty"),
                Self::StartWithDigit { name } => {
                    write!(f, "the name `{name}` cannot start with a digit")
                }
                Self::Start { first_char, name } => {
                    write!(
                        f,
                        "invalid character `{first_char}` in crate name: `{name}`, \
                         the first character must be an ASCII character"
                    )
                }
                Self::Char { ch, name } => {
                    write!(
                        f,
                        "invalid character `{ch}` in crate name: `{name}`, \
                         characters must be ASCII alphanumeric, `-`, or `_`"
                    )
                }
            }
        }
    }

    impl std::error::Error for InvalidCrateName {}

    pub fn validate_crate_name(name: &str) -> Result<(), InvalidCrateName> {
        if name.chars().count() > MAX_NAME_LENGTH {
            return Err(InvalidCrateName::TooLong { name: name.into() });
        }

        if name.is_empty() {
            return Err(InvalidCrateName::Empty);
        }

        let mut chars = name.chars();
        if let Some(ch) = chars.next() {
            if ch.is_ascii_digit() {
                return Err(InvalidCrateName::StartWithDigit { name: name.into() });
            }
            if !ch.is_ascii_alphabetic() {
                return Err(InvalidCrateName::Start {
                    first_char: ch,
                    name: name.into(),
                });
            }
        }

        for ch in chars {
            if !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') {
                return Err(InvalidCrateName::Char {
                    ch,
                    name: name.into(),
                });
            }
        }

        Ok(())
    }
}

// Reserved names from crates.io database migrations:
//   20170305095748_create_reserved_crate_names (Rust compiler internals)
//   20170430202433_reserve_windows_crate_names (Windows device names)
//   2021-02-10-141019_reserve_com0_lpt0 (com0, lpt0)
const RESERVED_NAMES: &[&str] = &[
    // Rust compiler internals
    "alloc",
    "arena",
    "ast",
    "builtins",
    "collections",
    "compiler-builtins",
    "compiler-rt",
    "compiletest",
    "core",
    "coretest",
    "debug",
    "driver",
    "flate",
    "fmt_macros",
    "grammar",
    "graphviz",
    "macro",
    "macros",
    "proc_macro",
    "rbml",
    "rust-installer",
    "rustbook",
    "rustc",
    "rustc_back",
    "rustc_borrowck",
    "rustc_driver",
    "rustc_llvm",
    "rustc_resolve",
    "rustc_trans",
    "rustc_typeck",
    "rustdoc",
    "rustllvm",
    "rustuv",
    "serialize",
    "std",
    "syntax",
    "test",
    "unicode",
    // Windows device names
    "nul",
    "con",
    "prn",
    "aux",
    "com0",
    "com1",
    "com2",
    "com3",
    "com4",
    "com5",
    "com6",
    "com7",
    "com8",
    "com9",
    "lpt0",
    "lpt1",
    "lpt2",
    "lpt3",
    "lpt4",
    "lpt5",
    "lpt6",
    "lpt7",
    "lpt8",
    "lpt9",
];

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum number of concurrent HTTP requests when checking names in bulk.
pub const MAX_CONCURRENT_REQUESTS: usize = 20;

static RESERVED_SET: LazyLock<HashSet<String>> =
    LazyLock::new(|| RESERVED_NAMES.iter().map(|s| canon_crate_name(s)).collect());

/// The availability status of a crate name on crates.io.
///
/// Returned as the success case of [`check_name`]. All three variants
/// represent a definitive answer from the index or validation layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
#[non_exhaustive]
pub enum Availability {
    /// The name is available for publishing.
    Available,
    /// The name is already taken by an existing crate (or a canonical collision).
    Taken,
    /// The name is reserved by crates.io (e.g., `std`, `core`, Windows device names).
    Reserved,
}

impl fmt::Display for Availability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "available"),
            Self::Taken => write!(f, "taken"),
            Self::Reserved => write!(f, "reserved"),
        }
    }
}

/// Errors that can occur when checking a crate name.
///
/// Returned as the error case of [`check_name`]. Implements
/// [`std::error::Error`] with proper [`source`](std::error::Error::source)
/// chaining.
#[derive(Debug)]
#[non_exhaustive]
pub enum CheckError {
    /// The crate name is syntactically invalid per crates.io rules.
    InvalidName(validation::InvalidCrateName),
    /// A network or HTTP error prevented querying the crates.io API.
    IndexLookup(Box<ureq::Error>),
    /// An internal error (e.g., thread panic) that prevented checking.
    Internal(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(e) => write!(f, "invalid: {e}"),
            Self::IndexLookup(e) => write!(f, "unknown: {e}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidName(e) => Some(e),
            Self::IndexLookup(e) => Some(e.as_ref()),
            Self::Internal(_) => None,
        }
    }
}

impl From<validation::InvalidCrateName> for CheckError {
    fn from(e: validation::InvalidCrateName) -> Self {
        Self::InvalidName(e)
    }
}

/// An HTTP client configured for crates.io API queries.
///
/// Wraps the underlying HTTP agent to insulate callers from the specific
/// HTTP library version used internally.
///
/// # Example
///
/// ```no_run
/// use cargo_avail::check::Client;
///
/// let client = Client::new();
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    agent: Agent,
}

impl Client {
    /// Create a new client with default timeout settings.
    #[must_use]
    pub fn new() -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION"),
                " (",
                env!("CARGO_PKG_REPOSITORY"),
                ")"
            ))
            .build();
        Self {
            agent: Agent::new_with_config(config),
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

/// Canonicalize a crate name: lowercase and replace hyphens with underscores.
///
/// crates.io treats `foo-bar` and `foo_bar` as the same crate name.
///
/// ```
/// use cargo_avail::check::canon_crate_name;
/// assert_eq!(canon_crate_name("My-Crate"), "my_crate");
/// ```
#[must_use]
pub fn canon_crate_name(name: &str) -> String {
    name.to_lowercase().replace('-', "_")
}

/// Check whether a crate name is available on crates.io.
///
/// Performs three checks in order:
/// 1. Validates the name against crates.io naming rules.
/// 2. Checks the name against the reserved names list.
/// 3. Queries the crates.io API, which uses the same canonical matching
///    as `cargo publish` (hyphens and underscores are equivalent).
///
/// # Errors
///
/// Returns [`CheckError::InvalidName`] if the name fails crates.io validation,
/// or [`CheckError::IndexLookup`] if the API cannot be queried.
///
/// # Example
///
/// ```no_run
/// use cargo_avail::check::{Client, check_name, Availability};
///
/// let client = Client::new();
/// match check_name(&client, "my-cool-crate") {
///     Ok(Availability::Available) => println!("go grab it!"),
///     Ok(status) => println!("{status}"),
///     Err(e) => eprintln!("error: {e}"),
/// }
/// ```
pub fn check_name(client: &Client, name: &str) -> Result<Availability, CheckError> {
    // 1. Validate using vendored crates.io logic
    validation::validate_crate_name(name)?;

    let canonical = canon_crate_name(name);

    // 2. Reserved names (checked against canonical form)
    if RESERVED_SET.contains(&canonical) {
        return Ok(Availability::Reserved);
    }

    // 3. crates.io API lookup
    // The API canonicalizes the name before querying (same logic as cargo publish),
    // so one request covers ALL separator variants. No need to guess which spelling
    // was used when the crate was published.
    let url = format!("https://crates.io/api/v1/crates/{canonical}");
    match client.agent.get(&url).call() {
        Ok(_) => Ok(Availability::Taken),
        Err(ureq::Error::StatusCode(404)) => Ok(Availability::Available),
        Err(e) => Err(CheckError::IndexLookup(Box::new(e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canon_crate_name_lowercases_and_replaces_hyphens() {
        assert_eq!(canon_crate_name("My-Crate"), "my_crate");
        assert_eq!(canon_crate_name("foo_bar"), "foo_bar");
        assert_eq!(canon_crate_name("FOO"), "foo");
    }

    #[test]
    fn reserved_set_contains_canonicalized() {
        assert!(RESERVED_SET.contains("std"));
        assert!(RESERVED_SET.contains("compiler_builtins"));
        assert!(RESERVED_SET.contains("rust_installer"));
    }

    #[test]
    fn reserved_windows_device_names() {
        assert!(RESERVED_SET.contains("nul"));
        assert!(RESERVED_SET.contains("con"));
        assert!(RESERVED_SET.contains("prn"));
        assert!(RESERVED_SET.contains("aux"));
        assert!(RESERVED_SET.contains("com0"));
        assert!(RESERVED_SET.contains("com9"));
        assert!(RESERVED_SET.contains("lpt0"));
        assert!(RESERVED_SET.contains("lpt9"));
    }

    #[test]
    fn invalid_name_returns_error() {
        let client = Client::new();
        match check_name(&client, "foo+bar") {
            Err(CheckError::InvalidName(e)) => {
                assert!(e.to_string().contains("invalid character"));
            }
            other => panic!("expected InvalidName, got {other:?}"),
        }
    }

    #[test]
    fn reserved_name_returns_reserved() {
        let client = Client::new();
        match check_name(&client, "std") {
            Ok(Availability::Reserved) => {}
            other => panic!("expected Reserved, got {other:?}"),
        }
    }

    #[test]
    fn reserved_name_canonical_match() {
        let client = Client::new();
        match check_name(&client, "Compiler-Builtins") {
            Ok(Availability::Reserved) => {}
            other => panic!("expected Reserved for canonical match, got {other:?}"),
        }
    }

    #[test]
    fn reserved_windows_name() {
        let client = Client::new();
        match check_name(&client, "NUL") {
            Ok(Availability::Reserved) => {}
            other => panic!("expected Reserved for Windows device name, got {other:?}"),
        }
    }

    #[test]
    #[ignore = "requires network access; run with: cargo test -- --ignored"]
    fn taken_name() {
        let client = Client::new();
        match check_name(&client, "serde") {
            Ok(Availability::Taken) => {}
            other => panic!("expected Taken, got {other:?}"),
        }
    }

    #[test]
    #[ignore = "requires network access"]
    fn available_name() {
        let client = Client::new();
        match check_name(&client, "zzzyyyxxxwww-not-a-real-crate") {
            Ok(Availability::Available) => {}
            other => panic!("expected Available, got {other:?}"),
        }
    }

    #[test]
    #[ignore = "requires network access"]
    fn canonical_collision_detected() {
        let client = Client::new();
        match check_name(&client, "tokio_util") {
            Ok(Availability::Taken) => {}
            other => panic!("expected Taken for canonical collision, got {other:?}"),
        }
    }

    #[test]
    #[ignore = "requires network access"]
    fn canonical_collision_via_api() {
        // This test verifies that the crates.io API canonical matching works.
        // `serde-json` is published as `serde_json` -- querying the canonical form
        // via the API should still find it.
        let client = Client::new();
        match check_name(&client, "serde-json") {
            Ok(Availability::Taken) => {}
            other => panic!("expected Taken for API canonical match, got {other:?}"),
        }
    }

    // Auto-trait compile-time tests (RFR Ch.3 Listing 3-8)
    #[test]
    fn availability_is_send_sync_unpin() {
        fn assert_normal<T: Sized + Send + Sync + Unpin>() {}
        assert_normal::<Availability>();
    }

    #[test]
    fn check_error_is_send_sync() {
        fn assert_normal<T: Sized + Send + Sync>() {}
        assert_normal::<CheckError>();
    }

    #[test]
    fn client_is_send_sync() {
        fn assert_normal<T: Sized + Send + Sync>() {}
        assert_normal::<Client>();
    }

    // Property-based tests
    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn canon_is_idempotent(name in "[a-zA-Z][a-zA-Z0-9_-]{0,63}") {
                let once = canon_crate_name(&name);
                let twice = canon_crate_name(&once);
                prop_assert_eq!(once, twice);
            }

            #[test]
            fn canon_has_no_hyphens_or_uppercase(name in "[a-zA-Z][a-zA-Z0-9_-]{0,63}") {
                let canonical = canon_crate_name(&name);
                prop_assert!(!canonical.contains('-'));
                prop_assert_eq!(&canonical, &canonical.to_lowercase());
            }

            #[test]
            fn separator_variants_share_canonical_form(
                base in "[a-z]{2,10}",
                sep_positions in proptest::collection::vec(0..10usize, 1..3)
            ) {
                // Build names with hyphens and underscores at the same positions
                let mut with_hyphens = base.clone();
                let mut with_underscores = base.clone();
                for &pos in &sep_positions {
                    let pos = pos.min(with_hyphens.len().saturating_sub(1));
                    if pos > 0 && pos < with_hyphens.len() {
                        with_hyphens.insert(pos, '-');
                        with_underscores.insert(pos, '_');
                    }
                }
                prop_assert_eq!(
                    canon_crate_name(&with_hyphens),
                    canon_crate_name(&with_underscores),
                );
            }
        }
    }
}
