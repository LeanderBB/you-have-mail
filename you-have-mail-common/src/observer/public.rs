use crate::observer::rpc::{AddAccountRequest, ObserverPRC, ObserverRequest, RemoveAccountRequest};
use crate::observer::worker::Worker;
use crate::{Account, AccountError, Notifier};
use proton_api_rs::tokio::sync::mpsc::Sender;
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
}

/// Errors returned during observer RPC calls.
#[derive(Debug, Error)]
pub enum ObserverRPCError<T> {
    #[error("Failed to send request to observer")]
    SendFailed(T),
    #[error("Failed to send request to observer, but did no manage to recover request type")]
    SendFailedUnexpectedType,
    #[error("No reply received for the request")]
    NoReply,
    #[error("{0}")]
    Error(#[from] ObserverError),
}

/// Errors that may occur during the [Observer](struct@Observer)'s execution.
#[derive(Debug, Error)]
pub enum ObserverError {
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

    /// Controlls the poll interval to check for new updates.
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

    pub async fn add_account(&self, account: Account) -> Result<(), ObserverRPCError<Account>> {
        if !account.is_logged_in() {
            return Err(ObserverRPCError::Error(ObserverError::AccountError(
                AccountError::InvalidState,
            )));
        }

        self.perform_rpc(AddAccountRequest { account }).await
    }

    pub async fn remove_account<T: Into<String>>(
        &self,
        email: T,
    ) -> Result<(), ObserverRPCError<String>> {
        self.perform_rpc(RemoveAccountRequest {
            email: email.into(),
        })
        .await
    }

    pub async fn shutdown_worker(&self) -> Result<(), ObserverRPCError<()>> {
        if self.0.send(ObserverRequest::Exit).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    pub async fn pause(&self) -> Result<(), ObserverRPCError<()>> {
        if self.0.send(ObserverRequest::Pause).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    pub async fn resume(&self) -> Result<(), ObserverRPCError<()>> {
        if self.0.send(ObserverRequest::Resume).await.is_err() {
            return Err(ObserverRPCError::SendFailed(()));
        }

        Ok(())
    }

    async fn perform_rpc<T: ObserverPRC>(
        &self,
        value: T,
    ) -> Result<T::Output, ObserverRPCError<T::SendFailedValue>> {
        let (sender, mut receiver) =
            proton_api_rs::tokio::sync::mpsc::channel::<Result<T::Output, ObserverError>>(1);
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
