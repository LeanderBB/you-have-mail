//! Observer Module, background worker that checks every active account.
mod public;
mod rpc;
mod worker;

pub use public::*;

#[cfg(test)]
mod observer_tests;
