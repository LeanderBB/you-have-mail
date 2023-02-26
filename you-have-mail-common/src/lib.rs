#![cfg_attr(docsrs, feature(doc_cfg))]
// Enable clippy if our Cargo.toml file asked us to do so.
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
// Enable as many useful Rust and Clippy warnings as we can stand.
#![warn(
    missing_copy_implementations,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    trivial_casts,
    unused_qualifications
)]
#![cfg_attr(feature = "clippy", warn(cast_possible_truncation))]
#![cfg_attr(feature = "clippy", warn(cast_possible_wrap))]
#![cfg_attr(feature = "clippy", warn(cast_precision_loss))]
#![cfg_attr(feature = "clippy", warn(cast_sign_loss))]
#![cfg_attr(feature = "clippy", warn(missing_docs_in_private_items))]
#![cfg_attr(feature = "clippy", warn(mut_mut))]
// Disallow `println!`. Use `debug!` for debug output
// (which is provided by the `log` crate).
#![cfg_attr(feature = "clippy", warn(print_stdout))]
// This allows us to use `unwrap` on `Option` values (because doing makes
// working with Regex matches much nicer) and when compiling in test mode
// (because using it in tests is idiomatic).
#![cfg_attr(all(not(test), feature = "clippy"), warn(result_unwrap_used))]
#![cfg_attr(feature = "clippy", warn(unseparated_literal_suffix))]
#![cfg_attr(feature = "clippy", warn(wrong_pub_self_convention))]

//! Shared for all the you-have-mail platforms/targets.

mod account;
pub mod backend;
mod config;
mod encryption;
mod observer;
mod traits;

pub use account::*;
pub use config::*;
pub use encryption::EncryptionKey;
pub use observer::*;
pub use traits::*;
