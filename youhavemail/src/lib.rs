//! Shared for all the you-have-mail platforms/targets.

pub mod backend;
pub mod encryption;
//mod observer;
pub mod db;
pub mod state;
pub mod yhm;

pub use secrecy;

pub mod events;
mod v1;
