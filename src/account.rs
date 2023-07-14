use crate::backend::{AuthRefresher, CheckTask};
use crate::Proxy;
use proton_api_rs::log::error;
use secrecy::SecretString;
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
    proxy: Option<Proxy>,
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
    #[error("Proxy configuration is invalid or Proxy is unreachable")]
    Proxy,
    #[error("Unknown error: {0}")]
    Unknown(anyhow::Error),
}

impl AccountError {
    pub fn is_logged_out(&self) -> bool {
        if let AccountError::Backend(e) = self {
            return matches!(e, crate::backend::BackendError::LoggedOut);
        }
        false
    }
}

pub type AccountResult<T> = Result<T, AccountError>;

impl Account {
    pub fn new<T: Into<String>>(
        backend: Arc<dyn crate::backend::Backend>,
        email: T,
        proxy: Option<Proxy>,
    ) -> Self {
        Self {
            backend,
            state: AccountState::LoggedOut,
            email: email.into(),
            proxy,
        }
    }

    pub fn with_state<T: Into<String>>(
        backend: Arc<dyn crate::backend::Backend>,
        email: T,
        state: AccountState,
        proxy: Option<Proxy>,
    ) -> Self {
        Self {
            backend,
            state,
            email: email.into(),
            proxy,
        }
    }

    /// Take ownership of the current account and put the state to LoggedOut on the the original
    /// instance.
    pub fn take(&mut self) -> Account {
        Account {
            backend: self.backend.clone(),
            email: self.email.clone(),
            state: std::mem::replace(&mut self.state, AccountState::LoggedOut),
            proxy: self.proxy.take(),
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

    /// Login to the account with the given password.
    pub fn login(&mut self, password: &SecretString, hv_data: Option<String>) -> AccountResult<()> {
        if !self.is_logged_out() {
            return Err(AccountError::InvalidState);
        }

        self.state = self
            .backend
            .login(&self.email, password, self.proxy.as_ref(), hv_data)?;
        Ok(())
    }

    /// Logout the current account.
    pub fn logout(&mut self) -> AccountResult<()> {
        let old_state = std::mem::replace(&mut self.state, AccountState::LoggedOut);
        match old_state {
            AccountState::LoggedOut | AccountState::AwaitingTotp(..) => Ok(()),
            AccountState::LoggedIn(mut account) => {
                if let Err(e) = account.logout() {
                    let _ = std::mem::replace(&mut self.state, AccountState::LoggedIn(account));
                    return Err(e.into());
                }
                Ok(())
            }
        }
    }

    /// Submit totp. If the account is not in the awaiting totp state, the
    /// `AccountError::InvalidState` error will be returned.
    pub fn submit_totp(&mut self, totp: &str) -> AccountResult<()> {
        match &mut self.state {
            AccountState::AwaitingTotp(t) => {
                let account = t.submit_totp(totp)?;
                self.state = AccountState::LoggedIn(account);
                Ok(())
            }
            _ => Err(AccountError::InvalidState),
        }
    }

    /// Refresh the authentication token for this account.
    pub fn refresh(&mut self, refresher: Box<dyn AuthRefresher>) -> AccountResult<()> {
        if !self.is_logged_out() {
            return Err(AccountError::InvalidState);
        }

        self.state = refresher.refresh(self.proxy.as_ref())?;
        Ok(())
    }

    /// Apply proxy configuration to this account
    pub fn set_proxy(&mut self, proxy: Option<&Proxy>) -> AccountResult<bool> {
        if self.proxy.as_ref() == proxy {
            return Ok(false);
        }

        if let Some(p) = proxy {
            self.backend.check_proxy(p).map_err(|e| {
                error!("Failed to apply proxy to account {}:{e}", self.email);
                AccountError::Proxy
            })?;
        }

        if let AccountState::LoggedIn(a) = &mut self.state {
            a.set_proxy(proxy)?;
        }

        self.proxy = proxy.cloned();

        Ok(true)
    }

    /// Get current proxy applied to this account.
    pub fn get_proxy(&self) -> &Option<Proxy> {
        &self.proxy
    }

    /// Get new instance of a task to run a check on the account to see if new emails have arrived.
    pub fn get_task(&self) -> AccountResult<Box<dyn CheckTask>> {
        match &self.state {
            AccountState::LoggedIn(a) => Ok(a.new_task()),
            _ => Err(AccountError::InvalidState),
        }
    }

    pub(crate) fn get_impl(&self) -> Option<&dyn crate::backend::Account> {
        if let AccountState::LoggedIn(a) = &self.state {
            Some(a.as_ref())
        } else {
            None
        }
    }
}
