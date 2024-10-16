//! You Have Mail Common contains all the shared code which powers
//! [You Have Mail Mobile](https://github.com/LeanderBB/you-have-mail/) application.
//!
//! It provides a polling mechanism which checks the email [`Backend`](backend::Backend) for new
//! messages and then generates a [`PollOutput`](yhm::PollOutput) which can be consumed by the
//! integrating application as required.
//!
//! All the data is persisted in a Sqlite database and can be accessed by different processes. Each
//! account has access to local state container and a secret state container, the latter is encrypted
//! using the provided encryption [`Key`](encryption::key). See [`Account`](state::Account) for
//! more details.
//!
//! # Adding a new Backend
//!
//! To add a new backend, first implement both the [`Backend`](backend::Backend) and the
//! [`Poller`](backend::Poller) traits. The [`Backend`](backend::Backend) can then be registered
//! with [`Yhm`](yhm::Yhm) at startup.
//!
//! To main the proxy support for the backend, all backends should (for the time being) implement
//! their network API over [`Client`](http::Client) type. See the proton-api crate as an example.
//!
//! Since each backend's login sequence is unique, they should be added as a dedicated type and
//! implement the [`IntoAccount`](yhm::IntoAccount`) trait to register the new account as soon as
//! the login has been completed.
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use you_have_mail_common::yhm::{Error, IntoAccount, Yhm};
//! struct NewLoginSequence{}
//!
//! #[derive(Serialize, Deserialize)]
//! struct State{}
//!
//! #[derive(Serialize, Deserialize)]
//! struct Secret{}
//!
//! impl IntoAccount for NewLoginSequence {
//!
//!     fn into_account(self, yhm: &Yhm) -> Result<(), Error> {
//!         // Create account
//!         let account = yhm.new_account("email", "backend_name")?;
//!
//!         // Setup initial state
//!         let state= State{};
//!         account.set_state(Some(&state))?;
//!
//!         // Save account secrets
//!         let secret = Secret{};
//!         account.set_secret(Some(&secret))?;
//!
//!         // Save proxy information if used during the process
//!         // account.set_proxy(..)
//!
//!         Ok(())
//!     }
//! }
//!
//! ```
//!
//! # Example
//!
//! ```rust
//! use sqlite_watcher::watcher::Watcher;
//! use you_have_mail_common::backend;
//! use you_have_mail_common::encryption::Key;
//! use you_have_mail_common::events::Event;
//! use you_have_mail_common::state::State;
//! use you_have_mail_common::yhm::{IntoAccount, Yhm};
//!
//! let db_path = "/tmp/state.db";
//! let encryption_key = Key::new();
//! let watcher = Watcher::new().unwrap();
//! // Create new state
//! let state = State::new(db_path.into(), encryption_key, watcher).unwrap();
//! let yhm = Yhm::new(state);
//!
//! // Add a new account.
//! let login_sequence = backend::proton::Backend::login_sequence(None).unwrap();
//! // do login ...
//!
//! // Register with Yhm - This only works if we are logged in.
//! //login_sequence.into_account(&yhm).unwrap();
//!
//! // Poll account
//! let poll_output = yhm.poll().unwrap();
//!
//! // Retrieve read the last poll events from the db
//! let events = yhm.last_events().unwrap();
//!
//! ```
//!
//!

pub mod backend;
pub mod encryption;
//mod observer;
pub mod db;
pub mod state;
pub mod yhm;

pub use secrecy;

pub mod events;

pub use http;
mod v1;
