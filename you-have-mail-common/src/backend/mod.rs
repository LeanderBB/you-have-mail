//! Implementations for possible account backends from which one can receive email
//! notifications for.

use crate::AccountState;
use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::fmt::Debug;
use thiserror::Error;

#[doc(hidden)]
pub mod null;

#[cfg(feature = "proton-backend")]
#[cfg_attr(docsrs, doc(cfg(feature = "proton-backend")))]
pub mod proton;

/// Expected backend errors.
#[derive(Debug, Error)]
pub enum BackendError {
    #[error("The user account has been logged out or the token expired")]
    LoggedOut,
    #[error("The user account server's are not reachable")]
    Offline,
    #[error("{0}")]
    Request(#[source] anyhow::Error),
    #[error("An unknown error occurred: {0}")]
    Unknown(#[source] anyhow::Error),
}

pub type BackendResult<T> = Result<T, BackendError>;

/// Reply for new email queries.
#[derive(Debug, Copy, Clone)]
pub struct NewEmailReply {
    pub count: usize,
}

/// Implementation for the backends.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Backend: Send + Sync + Debug {
    /// Return the backend's name.
    fn name(&self) -> &str;

    /// Return the backend's description
    fn description(&self) -> &str;

    /// Login an account.
    async fn login(&self, username: &str, password: &str) -> BackendResult<AccountState>;

    /// Load the necessary information to refresh the user's account access credentials.
    fn auth_refresher_from_config(
        &self,
        value: serde_json::Value,
    ) -> Result<Box<dyn AuthRefresher>, anyhow::Error>;
}

/// Trait that needs to be implemented for all backend accounts
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Account: Send + Sync + Debug {
    /// Execute the code that will check whether new mail is available.
    async fn check(&mut self) -> BackendResult<NewEmailReply>;

    /// Logout the account.
    async fn logout(&mut self) -> BackendResult<()>;

    /// Load the necessary information to refresh the user's account access credentials.
    fn auth_refresher_config(&self) -> Result<serde_json::Value, anyhow::Error>;
}

/// Trait for accounts that require 2FA support
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AwaitTotp: Send + Sync + Debug {
    /// Called when TOTP code will be submitted.
    async fn submit_totp(
        self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)>;
}

/// Trait to refresh the accounts' login credentials.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AuthRefresher: Send + Sync + Debug {
    async fn refresh(self: Box<Self>) -> Result<AccountState, BackendError>;
}
