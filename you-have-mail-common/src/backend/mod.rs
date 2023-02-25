//! Implementations for possible account backends from which one can receive email
//! notifications for.

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::fmt::Debug;
use thiserror::Error;

pub mod null;

#[cfg(feature = "proton-backend")]
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

    /// Login an account.
    async fn login(&self, username: &str, password: &str) -> BackendResult<crate::AccountState>;
}

/// Trait that needs to be implemented for all backend accounts
#[cfg_attr(test, automock)]
#[async_trait]
pub trait Account: Send + Debug {
    /// Execute the code that will check whether new mail is available.
    async fn check(&mut self) -> BackendResult<NewEmailReply>;

    /// Logout the account.
    async fn logout(&mut self) -> BackendResult<()>;
}

/// Trait for accounts that require 2FA support
#[cfg_attr(test, automock)]
#[async_trait]
pub trait AwaitTotp: Send + Debug {
    /// Called when TOTP code will be submitted.
    async fn submit_totp(
        self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)>;
}
