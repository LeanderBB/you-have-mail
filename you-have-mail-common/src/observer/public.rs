use crate::observer::rpc::{
    AddAccountRequest, GenConfigRequest, GetAccountListRequest, ObserverPRC, ObserverRequest,
    RemoveAccountRequest,
};
use crate::observer::worker::Worker;
use crate::{Account, AccountError, ConfigStoreError, EncryptionKey, Notifier};
use proton_api_rs::tokio::sync::mpsc::Sender;
use secrecy::Secret;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Clone)]
pub struct Observer(Arc<Sender<ObserverRequest>>);

/// Account status for the accounts being watched by the observer.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ObserverAccountStatus {
    /// Account's backend is offline.
    Offline,
    /// Account's backend is online and the account is logged in.
    Online,
    /// The account is logged out or the session expired.
    LoggedOut,
}

/// Account info for active accounts in the [Observer](struct@Observer).
#[derive(Debug, Clone)]
pub struct ObserverAccount {
    pub email: String,
    pub status: ObserverAccountStatus,
    pub backend: String,
}

/// Errors returned during observer RPC calls.
#[derive(Debug, Error)]
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

    pub fn build(self) -> (Observer, impl Future<Output = ()>) {
        Observer::new(self)
    }
}

impl Observer {
    fn new(builder: ObserverBuilder) -> (Self, impl Future<Output = ()>) {
        let (task, sender) = Worker::build(builder.notifier, builder.poll_interval);
        (Self(Arc::new(sender)), task)
    }

    /// Get the list of observed accounts and their status
    pub async fn get_accounts(
        &self,
    ) -> Result<Vec<ObserverAccount>, ObserverRPCError<(), ObserverError>> {
        self.perform_rpc(GetAccountListRequest {}).await
    }

    /// Add a new account to be observed for new emails.
    pub async fn add_account(
        &self,
        account: Account,
    ) -> Result<(), ObserverRPCError<Account, ObserverError>> {
        if !account.is_logged_in() {
            return Err(ObserverRPCError::Error(ObserverError::AccountError(
                AccountError::InvalidState,
            )));
        }

        self.perform_rpc(AddAccountRequest { account }).await
    }

    /// Remove an account with the following email from the observer list.
    pub async fn remove_account<T: Into<String>>(
        &self,
        email: T,
    ) -> Result<(), ObserverRPCError<String, ObserverError>> {
        self.perform_rpc(RemoveAccountRequest {
            email: email.into(),
        })
        .await
    }

    /// Signal that the worker should terminate.
    pub async fn shutdown_worker(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Exit).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Pause the execution of the observer.
    pub async fn pause(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Pause).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Resume the execution of the observer.
    pub async fn resume(&self) -> Result<(), ObserverRPCError<(), ObserverError>> {
        if self.0.send(ObserverRequest::Resume).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    /// Generate configuration data for the currently active account list.
    pub async fn generate_config(
        &self,
        key: Secret<EncryptionKey>,
    ) -> Result<Vec<u8>, ObserverRPCError<(), ConfigStoreError>> {
        self.perform_rpc(GenConfigRequest { key }).await
    }

    async fn perform_rpc<T: ObserverPRC>(
        &self,
        value: T,
    ) -> Result<T::Output, ObserverRPCError<T::SendFailedValue, T::Error>> {
        let (sender, mut receiver) =
            proton_api_rs::tokio::sync::mpsc::channel::<Result<T::Output, T::Error>>(1);
        let request = value.into_request(sender);
        if let Err(e) = self.0.send(request).await {
            if let Some(v) = T::recover_send_value(e.0) {
                return Err(ObserverRPCError::SendFailed(v));
            }

            return Err(ObserverRPCError::SendFailedUnexpectedType);
        }

        let Some(result) = receiver.recv().await else {
            return Err(ObserverRPCError::NoReply)
        };

        result.map_err(ObserverRPCError::Error)
    }
}
