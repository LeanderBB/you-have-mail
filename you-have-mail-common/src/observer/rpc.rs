use crate::{Account, ObserverAccount, ObserverError};
use proton_api_rs::tokio::sync::mpsc::Sender;

/// RPC Requests for the `Observer`.
pub enum ObserverRequest {
    Exit,
    AddAccount(Account, Sender<Result<(), ObserverError>>),
    RemoveAccount(String, Sender<Result<(), ObserverError>>),
    GetAccounts(Sender<Result<Vec<ObserverAccount>, ObserverError>>),
    Pause,
    Resume,
}

#[doc(hidden)]
pub trait ObserverPRC {
    type Output;
    type SendFailedValue;
    fn into_request(self, reply: ObserverRPCReply<Self::Output>) -> ObserverRequest;
    fn recover_send_value(request: ObserverRequest) -> Option<Self::SendFailedValue>;
}

#[doc(hidden)]
pub struct RemoveAccountRequest {
    pub email: String,
}

#[doc(hidden)]
type ObserverRPCReply<T> = Sender<Result<T, ObserverError>>;

#[doc(hidden)]
impl ObserverPRC for RemoveAccountRequest {
    type Output = ();
    type SendFailedValue = String;

    fn into_request(self, reply: ObserverRPCReply<Self::Output>) -> ObserverRequest {
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
    type SendFailedValue = Account;

    fn into_request(self, sender: ObserverRPCReply<Self::Output>) -> ObserverRequest {
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
    type SendFailedValue = ();

    fn into_request(self, reply: ObserverRPCReply<Self::Output>) -> ObserverRequest {
        ObserverRequest::GetAccounts(reply)
    }

    fn recover_send_value(_: ObserverRequest) -> Option<Self::SendFailedValue> {
        Some(())
    }
}
