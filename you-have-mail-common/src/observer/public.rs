use crate::observer::rpc::{
    AddAccountRequest, ApplyProxyRequest, GenConfigRequest, GetAccountListRequest,
    GetPollIntervalRequest, GetProxyRequest, LogoutAccountRequest, ObserverPRC, ObserverRequest,
    RemoveAccountRequest,
};
use crate::observer::worker::{BackgroundWorker, Worker};
use crate::{Account, AccountError, ConfigGenError, Notifier, Proxy};
use std::fmt::Formatter;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone)]
pub struct Observer(Arc<BackgroundWorker>);

/// Account status for the accounts being watched by the observer.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ObserverAccountStatus {
    /// Account's backend is offline.
    Offline,
    /// Account's backend is online and the account is logged in.
    Online,
    /// The account is logged out or the session expired.
    LoggedOut,
    /// The account encountered an error
    Error,
}

impl std::fmt::Display for ObserverAccountStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ObserverAccountStatus::Offline => {
                write!(f, "Offline")
            }
            ObserverAccountStatus::Online => {
                write!(f, "Online")
            }
            ObserverAccountStatus::LoggedOut => {
                write!(f, "LoggedOut")
            }
            ObserverAccountStatus::Error => {
                write!(f, "Error Occurred")
            }
        }
    }
}

/// Account info for active accounts in the [Observer](struct@Observer).
#[derive(Debug, Clone)]
pub struct ObserverAccount {
    pub email: String,
    pub status: ObserverAccountStatus,
    pub backend: String,
    pub proxy: Option<Proxy>,
}

/// Errors returned during observer RPC calls.
#[derive(Debug, Error)]
#[allow(clippy::result_large_err)]
pub enum ObserverRPCError<T, E> {
    #[error("Failed to send request to observer")]
    SendFailed(T),
    #[error("Failed to send request to observer, but did no manage to recover request type")]
    SendFailedUnexpectedType,
    #[error("No reply received for the request")]
    NoReply,
    #[error("{0}")]
    Error(#[from] E),
}

/// Errors that may occur during the [Observer](struct@Observer)'s execution.
#[derive(Debug, Error)]
pub enum ObserverError {
    #[error("The given account is already active {0:?}")]
    AccountAlreadyActive(Account),
    #[error("{0}")]
    AccountError(#[from] AccountError),
    #[error("Account {0} not found")]
    NoSuchAccount(String),
    #[error("Unknown error occurred: {0}")]
    Unknown(
        #[from]
        #[source]
        anyhow::Error,
    ),
}

pub struct ObserverBuilder {
    poll_interval: Duration,
    notifier: Box<dyn Notifier>,
}

impl ObserverBuilder {
    pub fn new(notifier: Box<dyn Notifier>) -> Self {
        Self {
            poll_interval: Duration::from_secs(30),
            notifier,
        }
    }

    /// Controls the poll interval to check for new updates.
    pub fn poll_interval(mut self, poll_interval: Duration) -> Self {
        self.poll_interval = poll_interval;
        self
    }

    pub fn build(self) -> Observer {
        Observer::new(self)
    }
}

impl Observer {
    fn new(builder: ObserverBuilder) -> Self {
        Self(Worker::build(builder.notifier, builder.poll_interval))
    }

    /// Get the list of observed accounts and their status
    pub fn get_accounts(
        &self,
    ) -> Result<Vec<ObserverAccount>, ObserverRPCError<(), ObserverError>> {
        self.perform_rpc(GetAccountListRequest {})
    }

    /// Add a new account to be observed for new emails.
    #[allow(clippy::result_large_err)]
    pub fn add_account(
        &self,
        account: Account,
    ) -> Result<(), ObserverRPCError<Account, ObserverError>> {
        self.perform_rpc(AddAccountRequest { account })
    }

    /// Logout an account, but do not remove it from the observer list
    pub fn logout_account<T: Into<String>>(
        &self,
        email: T,
    ) -> Result<(), ObserverRPCError<String, ObserverError>> {
        self.perform_rpc(LogoutAccountRequest {
            email: email.into(),
        })
    }

    /// Remove an account with the following email from the observer list.
    pub fn remove_account<T: Into<String>>(
        &self,
        email: T,
    ) -> Result<(), ObserverRPCError<String, ObserverError>> {
        self.perform_rpc(RemoveAccountRequest {
            email: email.into(),
        })
    }

    /// Signal that the worker should terminate.
    pub fn shutdown_worker(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Exit).is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Pause the execution of the observer.
    pub fn pause(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Pause).is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Resume the execution of the observer.
    pub fn resume(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Resume).is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Generate configuration data for the currently active account list.
    pub fn generate_config(&self) -> Result<String, ObserverRPCError<(), ConfigGenError>> {
        self.perform_rpc(GenConfigRequest {})
    }

    pub fn set_poll_interval(&self, duration: Duration) -> Result<(), ObserverRPCError<(), ()>> {
        if self
            .0
            .send(ObserverRequest::SetPollInterval(duration))
            .is_err()
        {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    pub fn get_poll_interval(&self) -> Result<Duration, ObserverRPCError<(), ()>> {
        self.perform_rpc(GetPollIntervalRequest {})
    }

    pub fn get_proxy_settings(
        &self,
        email: String,
    ) -> Result<Option<Proxy>, ObserverRPCError<(), ObserverError>> {
        self.perform_rpc(GetProxyRequest { email })
    }

    pub fn set_proxy_settings(
        &self,
        email: String,
        proxy: Option<Proxy>,
    ) -> Result<(), ObserverRPCError<(), ObserverError>> {
        self.perform_rpc(ApplyProxyRequest { email, proxy })
    }

    fn perform_rpc<T: ObserverPRC>(
        &self,
        value: T,
    ) -> Result<T::Output, ObserverRPCError<T::SendFailedValue, T::Error>> {
        let (sender, receiver) = crossbeam_channel::bounded::<Result<T::Output, T::Error>>(1);
        let request = value.into_request(sender);
        if let Err(e) = self.0.send(request) {
            if let Some(v) = T::recover_send_value(e.0) {
                return Err(ObserverRPCError::SendFailed(v));
            }

            return Err(ObserverRPCError::SendFailedUnexpectedType);
        }

        let Ok(result) = receiver.recv() else {
            return Err(ObserverRPCError::NoReply)
        };

        result.map_err(ObserverRPCError::Error)
    }
}
