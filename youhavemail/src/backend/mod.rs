//! Implementations for possible account backends from which one can receive email
//! notifications for.

use crate::state;
use crate::state::Account;
use http::{Client, Proxy};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

pub mod dummy;
pub mod proton;

/// Expected backend errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http: {0}")]
    Http(#[from] http::Error),
    #[error("Account session has expired")]
    SessionExpired,
    #[error("Db: {0}")]
    Db(#[from] state::Error),
    #[error("Unknown Backend: {0}")]
    UnknownBackend(String),
    #[error("An unknown error occurred: {0}")]
    Unknown(#[source] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Data type returned when a new email has been received.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NewEmail {
    /// Sender of the email.
    pub sender: String,
    /// Subject of the email.
    pub subject: String,
}

/// Implementation for the backends.
pub trait Backend: Send + Sync {
    /// Return the backend's name.
    fn name(&self) -> &str;

    /// Return the backend's description
    fn description(&self) -> &str;

    /// Create a new http client with the given `proxy` configuration.
    ///
    /// # Errors
    ///
    /// Should return an error if the client failed to build.
    fn create_client(&self, proxy: Option<Proxy>) -> Result<Arc<Client>>;

    /// Create a new [`Poller`] instance from the database `account` state.
    ///
    /// # Errors
    ///
    /// Should return error if we could not create the account.
    fn new_poller(&self, client: Arc<Client>, account: Account) -> Result<Box<dyn Poller>>;
}

/// Trait that needs to be implemented for all backend accounts
pub trait Poller {
    /// Check if there are new emails on the account.
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    fn check(&mut self) -> Result<Vec<NewEmail>>;

    /// Logout the account.
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    fn logout(&mut self) -> Result<()>;
}
