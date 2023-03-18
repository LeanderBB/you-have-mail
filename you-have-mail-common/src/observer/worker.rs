use crate::backend::BackendError;
use crate::observer::rpc::ObserverRequest;
use crate::{
    Account, AccountError, Config, Notification, Notifier, ObserverAccount, ObserverAccountStatus,
    ObserverError,
};
use proton_api_rs::log::{debug, error};
use proton_api_rs::tokio;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tokio::sync::mpsc::{Receiver, Sender};

/// Observer background worker. Handles RPC commands and polls the accounts for updates.
pub struct Worker {
    accounts: HashMap<String, WorkerAccount>,
    notifier: Box<dyn Notifier>,
    poll_interval: Duration,
    paused: bool,
}

/// Represents and active account.
struct WorkerAccount {
    account: Account,
    status: ObserverAccountStatus,
}

impl Worker {
    fn new(notifier: Box<dyn Notifier>, poll_interval: Duration) -> Self {
        Self {
            notifier,
            poll_interval,
            accounts: HashMap::new(),
            paused: false,
        }
    }

    pub fn build(
        notifier: Box<dyn Notifier>,
        poll_interval: Duration,
    ) -> (impl Future<Output = ()>, Sender<ObserverRequest>) {
        let (sender, receiver) = proton_api_rs::tokio::sync::mpsc::channel::<ObserverRequest>(5);
        let observer = Self::new(notifier, poll_interval);
        (observer_task(observer, receiver), sender)
    }

    #[cfg(test)]
    pub(crate) fn add_account(&mut self, account: Account) {
        self.accounts.insert(
            account.email().to_string(),
            WorkerAccount {
                account,
                status: ObserverAccountStatus::Online,
            },
        );
    }

    async fn handle_request(&mut self, request: ObserverRequest) -> bool {
        match request {
            ObserverRequest::AddAccount(account, reply) => {
                let account_status = account_status_to_observer_account_status(&account);
                debug!(
                    "Add account request: account {} Status={}",
                    account.email(),
                    account_status
                );
                let result = match self.accounts.entry(account.email().to_string()) {
                    Entry::Occupied(mut v) => {
                        if v.get().status == ObserverAccountStatus::LoggedOut {
                            self.notifier
                                .notify(Notification::AccountOnline(account.email()));
                            v.insert(WorkerAccount {
                                account,
                                status: account_status,
                            });
                            Ok(())
                        } else {
                            Err(ObserverError::AccountAlreadyActive(account))
                        }
                    }
                    Entry::Vacant(v) => {
                        self.notifier
                            .notify(Notification::AccountAdded(account.email()));
                        v.insert(WorkerAccount {
                            account,
                            status: account_status,
                        });
                        Ok(())
                    }
                };

                if reply.send(result).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::LogoutAccount(email, reply) => {
                debug!("Logout account request: account {email}");
                let result = if let Some(account) = self.accounts.get_mut(&email) {
                    let r = account.account.logout().await.map_err(|e| e.into());
                    if r.is_ok() {
                        account.status = ObserverAccountStatus::LoggedOut;
                        self.notifier.notify(Notification::AccountLoggedOut(&email));
                    }
                    r
                } else {
                    Err(ObserverError::NoSuchAccount(email))
                };

                if reply.send(result).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::RemoveAccount(email, reply) => {
                debug!("Remove account request: account {email}");
                let result = if let Some(mut account) = self.accounts.remove(&email) {
                    let r = account.account.logout().await;
                    if r.is_ok() {
                        self.notifier.notify(Notification::AccountRemoved(&email));
                    }
                    r
                } else {
                    Ok(())
                };

                if reply.send(result.map_err(|e| e.into())).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::GetAccounts(reply) => {
                debug!("Get accounts request");
                let accounts = self
                    .accounts
                    .iter()
                    .map(|(k, v)| ObserverAccount {
                        email: k.clone(),
                        status: v.status,
                        backend: v.account.backend().name().to_string(),
                    })
                    .collect::<Vec<_>>();

                if reply.send(Ok(accounts)).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::Exit => true,
            ObserverRequest::Pause => {
                debug!("Pause request");
                self.paused = true;
                false
            }
            ObserverRequest::Resume => {
                debug!("Resume request");
                self.paused = false;
                false
            }
            ObserverRequest::GenConfig(reply) => {
                debug!("Gen config request");
                let r = Config::store(self.accounts.values().map(|a| &a.account));

                if reply.send(r).await.is_err() {
                    error!("Failed to send reply for gen config request");
                }

                false
            }
            ObserverRequest::SetPollInterval(d) => {
                self.poll_interval = d;
                false
            }
            ObserverRequest::GetPollInterval(reply) => {
                if reply.send(Ok(self.poll_interval)).await.is_err() {
                    error!("Failed to send reply for poll interval request");
                }
                false
            }
        }
    }

    async fn poll_accounts(&mut self) {
        if self.paused {
            return;
        }

        for wa in &mut self.accounts.values_mut() {
            // Track logged out status if for some reason something slips through.
            if wa.account.is_logged_out() {
                wa.status = ObserverAccountStatus::LoggedOut;
            }

            // Skip accounts which are not logged in.
            if wa.status == ObserverAccountStatus::LoggedOut {
                continue;
            }

            debug!(
                "Polling account={} backend={}",
                wa.account.email(),
                wa.account.backend().name()
            );
            match wa.account.check().await {
                Ok(check) => {
                    if wa.status != ObserverAccountStatus::Online {
                        self.notifier
                            .notify(Notification::AccountOnline(wa.account.email()))
                    }
                    wa.status = ObserverAccountStatus::Online;
                    if check.count > 0 {
                        self.notifier.notify(Notification::NewEmail {
                            account: wa.account.email(),
                            backend: wa.account.backend().name(),
                            count: check.count,
                        });
                    }
                }
                Err(e) => {
                    error!(
                        "Poll failed account={} backend={}: {}",
                        wa.account.email(),
                        wa.account.backend().name(),
                        e
                    );
                    if let AccountError::Backend(be) = &e {
                        match be {
                            BackendError::LoggedOut => {
                                if wa.status == ObserverAccountStatus::LoggedOut {
                                    return;
                                }
                                self.notifier
                                    .notify(Notification::AccountLoggedOut(wa.account.email()));
                                wa.status = ObserverAccountStatus::LoggedOut;
                            }
                            BackendError::Offline => {
                                if wa.status == ObserverAccountStatus::Offline {
                                    return;
                                }
                                self.notifier
                                    .notify(Notification::AccountOffline(wa.account.email()));
                                wa.status = ObserverAccountStatus::Offline;
                            }
                            _ => self
                                .notifier
                                .notify(Notification::AccountError(wa.account.email(), e)),
                        }
                    }
                }
            }
        }
    }
}

async fn observer_task(mut observer: Worker, mut receiver: Receiver<ObserverRequest>) {
    debug!("Starting observer loop");
    let sleep = tokio::time::interval(observer.poll_interval);
    tokio::pin!(sleep);
    let mut last_poll_interval = observer.poll_interval;
    loop {
        // Update poll interval
        if last_poll_interval != observer.poll_interval {
            debug!("Updating observer poll interval old={:?} new={:?}", last_poll_interval, observer.poll_interval);
            let new_interval= tokio::time::interval(observer.poll_interval);
            *sleep = new_interval;
            last_poll_interval = observer.poll_interval;
        }

        tokio::select! {
            _ = sleep.tick() => {
                observer.poll_accounts().await;
            }

            request = receiver.recv() => {
                if let Some(request) = request {
                    if observer.handle_request(request).await {
                        break;
                    }
                }
            }

        }
    }
    debug!("Exiting observer loop")
}

#[cfg(test)]
mod tests {
    use crate::backend::{BackendError, MockAccount, NewEmailReply};
    use crate::observer::worker::Worker;
    use crate::{Account, AccountState, MockNotifier, Notification};
    use mockall::Sequence;
    use proton_api_rs::tokio;
    use std::time::Duration;

    #[tokio::test]
    async fn worker_notifies_offline_only_once() {
        let mut notifier = MockNotifier::new();
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountOffline(_)))
            .times(1)
            .return_const(());
        let mut mock_account = MockAccount::new();
        mock_account
            .expect_check()
            .times(4)
            .returning(|| Err(BackendError::Offline));
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // Poll account multiple times
        worker.poll_accounts().await;
        worker.poll_accounts().await;
        worker.poll_accounts().await;
        worker.poll_accounts().await;
    }

    #[tokio::test]
    async fn worker_notifies_offline_only_once_and_continues_once_account_comes_online() {
        let mut notifier = MockNotifier::new();

        let mut notifier_sequence = Sequence::new();
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountOffline(_)))
            .times(1)
            .in_sequence(&mut notifier_sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountOnline(_)))
            .times(1)
            .in_sequence(&mut notifier_sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::NewEmail { .. }))
            .times(1)
            .in_sequence(&mut notifier_sequence)
            .return_const(());
        let mut mock_account = MockAccount::new();
        let mut mock_sequence = Sequence::new();
        mock_account
            .expect_check()
            .times(2)
            .in_sequence(&mut mock_sequence)
            .returning(|| Err(BackendError::Offline));
        mock_account
            .expect_check()
            .times(1)
            .in_sequence(&mut mock_sequence)
            .returning(|| Ok(NewEmailReply { count: 1 }));
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // First two are offline
        worker.poll_accounts().await;
        worker.poll_accounts().await;
        // Last one should be online
        worker.poll_accounts().await;
    }

    #[tokio::test]
    async fn worker_notifies_logged_out_only_once_and_continues_offline() {
        let mut notifier = MockNotifier::new();
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountLoggedOut(_)))
            .times(1)
            .return_const(());
        notifier.expect_notify().times(0).return_const(());
        let mut mock_account = MockAccount::new();
        let mut mock_sequence = Sequence::new();
        mock_account
            .expect_check()
            .times(1)
            .in_sequence(&mut mock_sequence)
            .returning(|| Err(BackendError::LoggedOut));
        mock_account
            .expect_check()
            .times(0)
            .in_sequence(&mut mock_sequence)
            .returning(|| Ok(NewEmailReply { count: 1 }));
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // First all puts the account in logged in mode.
        worker.poll_accounts().await;
        // Next calls are noop.
        worker.poll_accounts().await;
        worker.poll_accounts().await;
    }
}

fn account_status_to_observer_account_status(account: &Account) -> ObserverAccountStatus {
    if account.is_logged_in() {
        ObserverAccountStatus::Online
    } else if account.is_logged_out() {
        ObserverAccountStatus::LoggedOut
    } else {
        ObserverAccountStatus::Offline
    }
}
