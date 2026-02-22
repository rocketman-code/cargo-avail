use std::collections::HashSet;
use std::time::Duration;

use ureq::Agent;

const RESERVED_NAMES: &[&str] = &[
    "alloc", "arena", "ast", "builtins", "collections", "compiler-builtins",
    "compiler-rt", "compiletest", "core", "coretest", "debug", "driver",
    "flate", "fmt_macros", "grammar", "graphviz", "macro", "macros",
    "proc_macro", "rbml", "rust-installer", "rustbook", "rustc", "rustc_back",
    "rustc_borrowck", "rustc_driver", "rustc_llvm", "rustc_resolve",
    "rustc_trans", "rustc_typeck", "rustdoc", "rustllvm", "rustuv",
    "serialize", "std", "syntax", "test", "unicode",
];

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

pub enum Status {
    Available,
    Taken,
    Reserved,
    Invalid(String),
    Unknown(String),
}

pub fn new_agent() -> Agent {
    let config = Agent::config_builder()
        .timeout_global(Some(REQUEST_TIMEOUT))
        .build();
    Agent::new_with_config(config)
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

pub fn check_name(agent: &Agent, name: &str) -> Status {
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
        match agent.get(&url).call() {
            // Ok means 2xx -- the crate exists in the index
            Ok(_) => return Status::Taken,
            // 404 means not found in the index -- continue checking other variants
            Err(ureq::Error::StatusCode(404)) => continue,
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
        let agent = new_agent();
        match check_name(&agent, "foo+bar") {
            Status::Invalid(msg) => assert!(msg.contains("invalid character")),
            _ => panic!("expected Invalid"),
        }
    }

    #[test]
    fn test_reserved_name() {
        let agent = new_agent();
        match check_name(&agent, "std") {
            Status::Reserved => {}
            _ => panic!("expected Reserved"),
        }
    }

    #[test]
    fn test_reserved_name_canonical_match() {
        let agent = new_agent();
        match check_name(&agent, "Compiler-Builtins") {
            Status::Reserved => {}
            _ => panic!("expected Reserved for canonical match"),
        }
    }

    #[test]
    fn test_taken_name() {
        let agent = new_agent();
        // serde definitely exists
        match check_name(&agent, "serde") {
            Status::Taken => {}
            _ => panic!("expected Taken"),
        }
    }

    #[test]
    fn test_available_name() {
        let agent = new_agent();
        // extremely unlikely to be taken
        match check_name(&agent, "zzzyyyxxxwww-not-a-real-crate") {
            Status::Available => {}
            _ => panic!("expected Available"),
        }
    }

    #[test]
    fn test_canonical_collision_detected() {
        let agent = new_agent();
        // tokio-util exists, so tokio_util should be taken
        match check_name(&agent, "tokio_util") {
            Status::Taken => {}
            _ => panic!("expected Taken for canonical collision"),
        }
    }
}
