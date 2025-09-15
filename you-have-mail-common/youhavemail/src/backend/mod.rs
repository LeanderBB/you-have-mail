//! Implementations for possible account backends from which one can receive email
//! notifications for.

use crate::state;
use crate::state::Account;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use you_have_mail_http::{Client, Proxy};

pub mod dummy;
pub mod proton;

/// Expected backend errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Http: {0}")]
    Http(#[from] you_have_mail_http::Error),
    #[error("Account session has expired")]
    SessionExpired,
    #[error("Db: {0}")]
    Db(#[from] state::Error),
    #[error("Unknown Backend: {0}")]
    UnknownBackend(String),
    #[error("An unknown error occurred: {0}")]
    Unknown(#[source] anyhow::Error),
    #[error("Action is not valid or not recognized")]
    InvalidAction,
}

pub type Result<T> = std::result::Result<T, Error>;

/// An action to be taken on an account.
///
/// Since this is specific to each backend implementation, we only
/// store a serialized metadata required for the account to execute this
/// action.
///
/// Note that this could have been implemented as a trait, but we require that
/// this information can be transferred over an FFI boundary and potentially
/// stored on disk.
///
/// It's not recommended to share secret information in actions.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Action(String);

impl Action {
    /// Create a new action from the serializable `value`
    ///
    /// # Errors
    ///
    /// Return error if the type can't be encoded into JSON.
    pub fn new<T: Serialize>(value: &T) -> std::result::Result<Self, serde_json::Error> {
        Ok(Self(serde_json::to_string(value)?))
    }

    /// Convert the action data back into a usable type.
    ///
    /// # Errors
    ///
    /// Return error if the type can't be decoded from JSON.
    pub fn to_value<'de, T: Deserialize<'de>>(
        &'de self,
    ) -> std::result::Result<T, serde_json::Error> {
        serde_json::from_str(self.0.as_str())
    }

    /// Create a new action from encoded `data`.
    #[must_use]
    pub fn with(data: String) -> Self {
        Self(data)
    }

    /// Consume this instance and retrieve the encoded data.
    #[must_use]
    pub fn take(self) -> String {
        self.0
    }
}

/// Data type returned when a new email has been received.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NewEmail {
    /// Sender of the email.
    pub sender: String,
    /// Subject of the email.
    pub subject: String,
    /// Encoded data to move this message to trash
    pub move_to_trash_action: Option<Action>,
    /// Encoded data to mark this message as read.
    pub mark_as_read_action: Option<Action>,
    /// Encoded data to move this message to spam
    pub move_to_spam_action: Option<Action>,
}

/// Implementation for the backends.
pub trait Backend: Send + Sync {
    /// Return the backend's name.
    fn name(&self) -> &'static str;

    /// Return the backend's description
    fn description(&self) -> &str;

    /// Create a new you-have-mail-http client with the given `proxy` configuration.
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

    /// Execute the given `action`
    ///
    /// # Errors
    ///
    /// Return error if the action could not be executed.
    fn apply(&mut self, action: &Action) -> Result<()>;

    /// Logout the account.
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    fn logout(&mut self) -> Result<()>;
}
