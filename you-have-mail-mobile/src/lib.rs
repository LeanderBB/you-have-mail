#![allow(clippy::module_name_repetitions)] // hard to enforce over binding layer.

//! You Have Mail bindings for mobile platforms.
pub mod proxy;

mod account;
pub mod android;
pub mod backend;
mod events;
mod logging;
mod watcher;
pub mod yhm;

uniffi::setup_scaffolding!();
