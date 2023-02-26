use crate::backend::{AuthRefresher, NewEmailReply};
use proton_api_rs::log::error;
use std::sync::Arc;
use thiserror::Error;

/// Represents a user account. While it would have been more idiomatic to have the account
/// represent as an enum, the code is meant to be used with other language bindings where
/// such a thing may not be possible.
/// User [Backend::login](fn@crate::backend::Backend::login) to obtain a new account.
#[derive(Debug)]
pub struct Account {
    backend: Arc<dyn crate::backend::Backend>,
    state: AccountState,
    email: String,
}

/// Possible states for an account.
#[derive(Debug)]
pub enum AccountState {
    /// The account has been logged out.
    LoggedOut,
    /// The account is awaiting 2FA TOTP code.
    AwaitingTotp(Box<dyn crate::backend::AwaitTotp>),
    /// The account is fully logged in.
    LoggedIn(Box<dyn crate::backend::Account>),
}

/// Error returned from account operations.
#[derive(Debug, Error)]
pub enum AccountError {
    #[error("Account is not in right state for this operation")]
    InvalidState,
    #[error("Backend error occurred: {0}")]
    Backend(#[from] crate::backend::BackendError),
}

impl AccountError {
    pub fn is_offline(&self) -> bool {
        if let AccountError::Backend(e) = self {
            return matches!(e, crate::backend::BackendError::Offline);
        }
        false
    }

    pub fn is_logged_out(&self) -> bool {
        if let AccountError::Backend(e) = self {
            return matches!(e, crate::backend::BackendError::LoggedOut);
        }
        false
    }
}

pub type AccountResult<T> = Result<T, AccountError>;

impl Account {
    pub fn new<T: Into<String>>(backend: Arc<dyn crate::backend::Backend>, email: T) -> Self {
        Self {
            backend,
            state: AccountState::LoggedOut,
            email: email.into(),
        }
    }

    pub fn with_state<T: Into<String>>(
        backend: Arc<dyn crate::backend::Backend>,
        email: T,
        state: AccountState,
    ) -> Self {
        Self {
            backend,
            state,
            email: email.into(),
        }
    }

    /// Whether the account is logged in.
    pub fn is_logged_in(&self) -> bool {
        matches!(self.state, AccountState::LoggedIn(..))
    }

    /// Whether the account is logged out.
    pub fn is_logged_out(&self) -> bool {
        matches!(self.state, AccountState::LoggedOut)
    }

    /// Whether the account is awaiting totp.
    pub fn is_awaiting_totp(&self) -> bool {
        matches!(self.state, AccountState::AwaitingTotp(..))
    }

    /// The account's email.
    pub fn email(&self) -> &str {
        &self.email
    }

    /// Get the account's backend.
    pub fn backend(&self) -> &dyn crate::backend::Backend {
        self.backend.as_ref()
    }

    /// Run check on the account to see if new emails have arrived.
    pub async fn check(&mut self) -> AccountResult<NewEmailReply> {
        match &mut self.state {
            AccountState::LoggedIn(a) => match a.check().await {
                Ok(r) => Ok(r),
                Err(e) => {
                    if matches!(e, crate::backend::BackendError::LoggedOut) {
                        self.state = AccountState::LoggedOut;
                    }
                    Err(e.into())
                }
            },
            _ => Err(AccountError::InvalidState),
        }
    }

    /// Login to the account with the given password.
    pub async fn login(&mut self, password: &str) -> AccountResult<()> {
        if !self.is_logged_out() {
            return Err(AccountError::InvalidState);
        }

        self.state = self.backend.login(&self.email, password).await?;
        Ok(())
    }

    /// Logout the current account.
    pub async fn logout(&mut self) -> AccountResult<()> {
        let old_state = std::mem::replace(&mut self.state, AccountState::LoggedOut);
        match old_state {
            AccountState::LoggedOut | AccountState::AwaitingTotp(..) => Ok(()),
            AccountState::LoggedIn(mut account) => {
                if let Err(e) = account.logout().await {
                    let _ = std::mem::replace(&mut self.state, AccountState::LoggedIn(account));
                    return Err(e.into());
                }
                Ok(())
            }
        }
    }

    /// Submit totp. If the account is not in the awaiting totp state, the
    /// `AccountError::InvalidState` error will be returned.
    pub async fn submit_totp(&mut self, totp: &str) -> AccountResult<()> {
        let old_state = std::mem::replace(&mut self.state, AccountState::LoggedOut);
        match old_state {
            AccountState::LoggedOut => Err(AccountError::InvalidState),
            AccountState::AwaitingTotp(t) => match t.submit_totp(totp).await {
                Ok(a) => {
                    self.state = AccountState::LoggedIn(a);
                    Ok(())
                }
                Err((t, e)) => {
                    let _ = std::mem::replace(&mut self.state, AccountState::AwaitingTotp(t));
                    Err(e.into())
                }
            },
            AccountState::LoggedIn(a) => {
                let _ = std::mem::replace(&mut self.state, AccountState::LoggedIn(a));
                Err(AccountError::InvalidState)
            }
        }
    }

    /// Refresh the authentication token for this account.
    pub async fn refresh(&mut self, refresher: Box<dyn AuthRefresher>) -> AccountResult<()> {
        if !self.is_logged_out() {
            return Err(AccountError::InvalidState);
        }

        self.state = refresher.refresh().await?;
        Ok(())
    }

    pub(crate) fn get_impl(&self) -> Option<&dyn crate::backend::Account> {
        if let AccountState::LoggedIn(a) = &self.state {
            Some(a.as_ref())
        } else {
            None
        }
    }
}
