//! Implementations for possible account backends from which one can receive email
//! notifications for.

use crate::{AccountState, Proxy};
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
    // Note: This is specific to each backend, consult each backend for more info.
    #[error("Human Verification Captcha Requested")]
    HVCaptchaRequest(String),
    #[error("Invalid Human Verification Data Supplied")]
    HVDataInvalid(#[source] anyhow::Error),
    #[error("The user account has been logged out or the token expired")]
    LoggedOut,
    #[error("The request or connection timed out: {0}")]
    Timeout(#[source] anyhow::Error),
    #[error("Connection error: {0}")]
    Connection(#[source] anyhow::Error),
    #[error("Encode/Decode error: {0}")]
    EncodeOrDecode(#[source] anyhow::Error),
    #[error("{0}")]
    Request(#[source] anyhow::Error),
    #[error("{0}")]
    API(#[source] anyhow::Error),
    #[error("An unknown error occurred: {0}")]
    Unknown(#[source] anyhow::Error),
}

pub type BackendResult<T> = Result<T, BackendError>;

#[derive(Debug, Clone)]
pub struct EmailInfo {
    pub sender: String,
    pub subject: String,
}
/// Reply for new email queries.
#[derive(Debug, Clone)]
pub struct NewEmailReply {
    pub emails: Vec<EmailInfo>,
}

/// Implementation for the backends.
#[cfg_attr(test, automock)]
pub trait Backend: Send + Sync + Debug {
    /// Return the backend's name.
    fn name(&self) -> &str;

    /// Return the backend's description
    fn description(&self) -> &str;

    /// Login an account.
    #[allow(clippy::needless_lifetimes)] // required for automock.
    fn login<'a>(
        &self,
        username: &str,
        password: &str,
        proxy: Option<&'a Proxy>,
        hv_data: Option<String>,
    ) -> BackendResult<AccountState>;

    /// Check proxy settings.
    fn check_proxy(&self, proxy: &Proxy) -> BackendResult<()>;

    /// Load the necessary information to refresh the user's account access credentials.
    fn auth_refresher_from_config(
        &self,
        value: serde_json::Value,
    ) -> Result<Box<dyn AuthRefresher>, anyhow::Error>;
}

/// Trait that needs to be implemented for all backend accounts
#[cfg_attr(test, automock)]
pub trait Account: Send + Sync + Debug {
    /// Execute the code that will check whether new mail is available.
    /// If the account token was refreshed the second member of the tuple will be true.
    fn check(&mut self) -> (BackendResult<NewEmailReply>, bool);

    /// Logout the account.
    fn logout(&mut self) -> BackendResult<()>;

    /// Apply the given proxy to the connector. If proxy is none, remove it.
    #[allow(clippy::needless_lifetimes)] // required for automock.
    fn set_proxy<'a>(&mut self, proxy: Option<&'a Proxy>) -> BackendResult<()>;

    /// Load the necessary information to refresh the user's account access credentials.
    fn auth_refresher_config(&self) -> Result<serde_json::Value, anyhow::Error>;
}

/// Trait for accounts that require 2FA support
#[cfg_attr(test, automock)]
pub trait AwaitTotp: Send + Sync + Debug {
    /// Called when TOTP code will be submitted.
    fn submit_totp(
        self: Box<Self>,
        totp: &str,
    ) -> Result<Box<dyn Account>, (Box<dyn AwaitTotp>, BackendError)>;
}

/// Trait to refresh the accounts' login credentials.
#[cfg_attr(test, automock)]
pub trait AuthRefresher: Send + Sync + Debug {
    #[allow(clippy::needless_lifetimes)] // required for automock.
    fn refresh<'a>(self: Box<Self>, proxy: Option<&'a Proxy>)
        -> Result<AccountState, BackendError>;
}
