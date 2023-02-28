//! Definitions of all the callbacks expected to be provided by the mobile clients.

use crate::ServiceError;
use you_have_mail_common::{Account, AccountError};

///  Trait through which notifications will be delivered to the mobile clients.
pub trait Notifier: Send + Sync {
    fn notify(&self, email: String, message_count: u64);

    fn notify_error(&self, email: String, error: ServiceError);
}

pub struct NotifierWrapper(pub Box<dyn Notifier>);
impl you_have_mail_common::Notifier for NotifierWrapper {
    fn notify(&self, account: &Account, email_count: usize) {
        self.0
            .notify(account.email().to_string(), email_count as u64);
    }

    fn notify_error(&self, email: &str, error: AccountError) {
        self.0.notify_error(email.to_string(), error.into());
    }
}

/// Trait through which service config load errors will be reported.
pub trait ServiceFromConfigCallback: Send + Sync {
    fn notify_error(&self, email: String, error: ServiceError);
}
