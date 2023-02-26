use crate::{Account, ConfigStoreError, EncryptionKey, ObserverAccount, ObserverError};
use proton_api_rs::tokio::sync::mpsc::Sender;
use secrecy::Secret;

/// RPC Requests for the `Observer`.
pub enum ObserverRequest {
    Exit,
    AddAccount(Account, Sender<Result<(), ObserverError>>),
    RemoveAccount(String, Sender<Result<(), ObserverError>>),
    GetAccounts(Sender<Result<Vec<ObserverAccount>, ObserverError>>),
    Pause,
    Resume,
    GenConfig(
        Secret<EncryptionKey>,
        Sender<Result<Box<[u8]>, ConfigStoreError>>,
    ),
}

#[doc(hidden)]
pub trait ObserverPRC {
    type Output;
    type Error;
    type SendFailedValue;
    fn into_request(self, reply: Sender<Result<Self::Output, Self::Error>>) -> ObserverRequest;
    fn recover_send_value(request: ObserverRequest) -> Option<Self::SendFailedValue>;
}

#[doc(hidden)]
pub struct RemoveAccountRequest {
    pub email: String,
}

#[doc(hidden)]
impl ObserverPRC for RemoveAccountRequest {
    type Output = ();
    type Error = ObserverError;
    type SendFailedValue = String;

    fn into_request(self, reply: Sender<Result<Self::Output, Self::Error>>) -> ObserverRequest {
        ObserverRequest::RemoveAccount(self.email, reply)
    }

    fn recover_send_value(r: ObserverRequest) -> Option<Self::SendFailedValue> {
        match r {
            ObserverRequest::RemoveAccount(s, _) => Some(s),
            _ => None,
        }
    }
}

#[doc(hidden)]
pub struct AddAccountRequest {
    pub account: Account,
}

impl ObserverPRC for AddAccountRequest {
    type Output = ();
    type Error = ObserverError;
    type SendFailedValue = Account;

    fn into_request(self, sender: Sender<Result<Self::Output, Self::Error>>) -> ObserverRequest {
        ObserverRequest::AddAccount(self.account, sender)
    }

    fn recover_send_value(r: ObserverRequest) -> Option<Self::SendFailedValue> {
        match r {
            ObserverRequest::AddAccount(a, _) => Some(a),
            _ => None,
        }
    }
}

#[doc(hidden)]
pub struct GetAccountList {}

impl ObserverPRC for GetAccountList {
    type Output = Vec<ObserverAccount>;
    type Error = ObserverError;
    type SendFailedValue = ();

    fn into_request(self, reply: Sender<Result<Self::Output, Self::Error>>) -> ObserverRequest {
        ObserverRequest::GetAccounts(reply)
    }

    fn recover_send_value(_: ObserverRequest) -> Option<Self::SendFailedValue> {
        Some(())
    }
}

#[doc(hidden)]
pub struct GenConfigRequest {
    pub key: Secret<EncryptionKey>,
}

impl ObserverPRC for GenConfigRequest {
    type Output = Box<[u8]>;
    type Error = ConfigStoreError;
    type SendFailedValue = ();

    fn into_request(self, reply: Sender<Result<Self::Output, Self::Error>>) -> ObserverRequest {
        ObserverRequest::GenConfig(self.key, reply)
    }

    fn recover_send_value(_: ObserverRequest) -> Option<Self::SendFailedValue> {
        Some(())
    }
}
