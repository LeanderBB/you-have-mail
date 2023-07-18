use crate::{Notification, Notifier};
use parking_lot::Mutex;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum AccountState {
    Online,
    LoggedOut,
    Offline,
}

/// Stateful notifier intercepts notification request and only allows certain notifications to pass
/// through if they do not repeat. E.g.: if an account is offline, there's no need to send
/// subsequent offline notifications if nothing changes.
pub struct StatefulNotifier {
    notifier: Arc<dyn Notifier>,
    account_state: Mutex<HashMap<String, AccountState>>,
}

impl StatefulNotifier {
    pub fn new(notifier: Arc<dyn Notifier>) -> Self {
        Self {
            notifier,
            account_state: Default::default(),
        }
    }

    pub fn account_request_succeed(&self, email: &str) {
        let mut accessor = self.account_state.lock();
        if let Some(state) = accessor.get_mut(email) {
            if *state != AccountState::Online {
                self.notifier.notify(Notification::AccountOnline(email));
                *state = AccountState::Online;
            }
        }
    }
}

impl Notifier for StatefulNotifier {
    fn notify(&self, notification: Notification) {
        match notification {
            Notification::NewEmail { account: email, .. } => {
                match self.account_state.lock().entry(email.to_string()) {
                    Entry::Occupied(mut o) => {
                        if *o.get() != AccountState::Online {
                            self.notifier.notify(Notification::AccountOnline(email));
                        }
                        o.insert(AccountState::Online);
                    }
                    Entry::Vacant(v) => {
                        v.insert(AccountState::Online);
                    }
                };
            }
            Notification::AccountAdded(email, _, _) => {
                self.account_state
                    .lock()
                    .insert(email.to_string(), AccountState::Online);
            }
            Notification::AccountLoggedOut(email) => {
                match self.account_state.lock().entry(email.to_string()) {
                    Entry::Occupied(mut o) => {
                        if *o.get() == AccountState::LoggedOut {
                            return;
                        }
                        o.insert(AccountState::LoggedOut);
                    }
                    Entry::Vacant(v) => {
                        v.insert(AccountState::LoggedOut);
                    }
                };
            }
            Notification::AccountRemoved(email) => {
                self.account_state.lock().remove(email);
            }
            Notification::AccountOffline(email) => {
                match self.account_state.lock().entry(email.to_string()) {
                    Entry::Occupied(mut o) => {
                        if *o.get() == AccountState::Offline {
                            return;
                        }
                        o.insert(AccountState::Offline);
                    }
                    Entry::Vacant(v) => {
                        v.insert(AccountState::Offline);
                    }
                };
            }
            Notification::AccountOnline(email) => {
                match self.account_state.lock().entry(email.to_string()) {
                    Entry::Occupied(mut o) => {
                        if *o.get() == AccountState::Online {
                            return;
                        }
                        o.insert(AccountState::Online);
                    }
                    Entry::Vacant(v) => {
                        v.insert(AccountState::Online);
                    }
                };
            }
            _ => {}
        };

        self.notifier.notify(notification);
    }
}
