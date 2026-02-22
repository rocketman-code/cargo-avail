#![warn(missing_docs, missing_debug_implementations)]

//! Check whether crate names are available on crates.io.
//!
//! This library validates crate names against the real crates.io rules
//! (vendored from the official `crates_io_validation` crate), checks them
//! against the reserved names list, and queries the sparse index to determine
//! availability -- including canonical collision detection where hyphens and
//! underscores are equivalent.
//!
//! # Example
//!
//! ```no_run
//! use cargo_avail::check::{Client, check_name, Availability};
//!
//! let client = Client::new();
//! match check_name(&client, "my-cool-crate") {
//!     Ok(Availability::Available) => println!("Name is available!"),
//!     Ok(Availability::Taken) => println!("Already taken."),
//!     Ok(Availability::Reserved) => println!("Reserved by crates.io."),
//!     Err(e) => eprintln!("Error: {e}"),
//!     _ => {} // non_exhaustive
//! }
//! ```

pub mod check;
