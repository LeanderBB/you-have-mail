use crate::backend::NewEmailReply;
use proton_api_rs::log::error;
use thiserror::Error;

/// Represents a user account. While it would have been more idiomatic to have the account
/// represent as an enum, the code is meant to be used with other language bindings where
/// such a thing may not be possible.
#[derive(Debug)]
pub struct Account {
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

pub type AccountResult<T> = Result<T, AccountError>;

impl Account {
    /// Login to a new account for a given backend.
    pub async fn login(
        backend: &dyn crate::backend::Backend,
        email: &str,
        password: &str,
    ) -> AccountResult<Self> {
        let account_state = backend.login(email, password).await?;
        Ok(Self {
            state: account_state,
            email: email.into(),
        })
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

    /// Run check on the account to see if new emails have arrived.
    pub async fn check(&mut self) -> AccountResult<NewEmailReply> {
        match &mut self.state {
            AccountState::LoggedIn(a) => {
                let e = a.check().await?;
                Ok(e)
            }
            _ => Err(AccountError::InvalidState),
        }
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
}
