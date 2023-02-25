//! Collection of Traits expected to be implemented by the respective application targets.

use crate::{Account, AccountError};
#[cfg(test)]
use mockall::automock;

/// When an email has been received the notifier will be called.
#[cfg_attr(test, automock)]
pub trait Notifier: Send + Sync {
    /// The given account has received `email_count` new emails since the last check.
    fn notify(&self, account: &Account, email_count: usize);

    /// Notifications for when account status changes.
    fn notify_error(&self, email: &str, error: AccountError);
}
