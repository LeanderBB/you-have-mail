//! Definitions of all the callbacks expected to be provided by the mobile clients.

use crate::{Proxy, ServiceError};

///  Trait through which notifications will be delivered to the mobile clients.
pub trait Notifier: Send + Sync {
    fn new_email(&self, account: String, backend: String, count: u32);
    fn account_added(&self, email: String);
    fn account_logged_out(&self, email: String);
    fn account_removed(&self, email: String);
    fn account_offline(&self, email: String);
    fn account_online(&self, email: String);
    fn account_error(&self, email: String, error: ServiceError);
    fn proxy_applied(&self, email: String, proxy: Option<Proxy>);
    fn account_token_refreshed(&self, email: String);
}

pub struct NotifierWrapper(pub Box<dyn Notifier>);
impl you_have_mail_common::Notifier for NotifierWrapper {
    fn notify(&self, notification: you_have_mail_common::Notification) {
        use you_have_mail_common::Notification as Not;
        match notification {
            Not::NewEmail {
                account,
                backend,
                count,
            } => {
                self.0
                    .new_email(account.to_string(), backend.to_string(), count as u32);
            }
            Not::AccountAdded(e) => self.0.account_added(e.to_string()),
            Not::AccountLoggedOut(e) => self.0.account_logged_out(e.to_string()),
            Not::AccountRemoved(e) => self.0.account_removed(e.to_string()),
            Not::AccountOffline(e) => self.0.account_offline(e.to_string()),
            Not::AccountOnline(e) => self.0.account_online(e.to_string()),
            Not::AccountError(e, err) => self.0.account_error(e.to_string(), err.into()),
            Not::ProxyApplied(e, proxy) => self.0.proxy_applied(e.to_string(), proxy.cloned()),
            Not::AccountTokenRefresh(e) => self.0.account_token_refreshed(e.to_string()),
        }
    }
}

/// Trait through which service config load errors will be reported.
pub trait ServiceFromConfigCallback: Send + Sync {
    fn notify_error(&self, email: String, error: ServiceError);
}
