use crate::backend::BackendError;
use crate::observer::rpc::ObserverRequest;
use crate::{Account, AccountError, Notifier, ObserverAccount, ObserverAccountStatus};
use proton_api_rs::log::error;
use proton_api_rs::tokio;
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
                self.accounts.insert(
                    account.email().to_string(),
                    WorkerAccount {
                        account,
                        status: ObserverAccountStatus::Online,
                    },
                );

                if reply.send(Ok(())).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::RemoveAccount(email, reply) => {
                let result = if let Some(mut account) = self.accounts.remove(&email) {
                    account.account.logout().await
                } else {
                    Ok(())
                };

                if reply.send(result.map_err(|e| e.into())).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::GetAccounts(reply) => {
                let accounts = self
                    .accounts
                    .iter()
                    .map(|(k, v)| ObserverAccount {
                        email: k.clone(),
                        status: v.status,
                    })
                    .collect::<Vec<_>>();

                if reply.send(Ok(accounts)).await.is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::Exit => true,
            ObserverRequest::Pause => {
                self.paused = true;
                false
            }
            ObserverRequest::Resume => {
                self.paused = false;
                false
            }
        }
    }

    async fn poll_accounts(&mut self) {
        if self.paused {
            return;
        }

        for (email, wa) in &mut self.accounts {
            // Track logged out status if for some reason something slips through.
            if wa.account.is_logged_out() {
                wa.status = ObserverAccountStatus::LoggedOut;
            }

            // Skip accounts which are not logged in.
            if wa.status == ObserverAccountStatus::LoggedOut {
                continue;
            }

            match wa.account.check().await {
                Ok(check) => {
                    wa.status = ObserverAccountStatus::Online;
                    if check.count > 0 {
                        self.notifier.notify(&wa.account, check.count);
                    }
                }
                Err(e) => {
                    if let AccountError::Backend(be) = &e {
                        match be {
                            BackendError::LoggedOut => {
                                if wa.status == ObserverAccountStatus::LoggedOut {
                                    return;
                                }
                                wa.status = ObserverAccountStatus::LoggedOut;
                            }
                            BackendError::Offline => {
                                if wa.status == ObserverAccountStatus::Offline {
                                    return;
                                }
                                wa.status = ObserverAccountStatus::Offline;
                            }
                            _ => {}
                        }
                    }
                    self.notifier.notify_error(email, e);
                }
            }
        }
    }
}

async fn observer_task(mut observer: Worker, mut receiver: Receiver<ObserverRequest>) {
    let sleep = tokio::time::interval(observer.poll_interval);
    tokio::pin!(sleep);
    loop {
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
}

#[cfg(test)]
mod tests {
    use crate::backend::{BackendError, MockAccount, NewEmailReply};
    use crate::observer::worker::Worker;
    use crate::{Account, AccountError, AccountState, MockNotifier};
    use mockall::Sequence;
    use proton_api_rs::tokio;
    use std::time::Duration;

    #[tokio::test]
    async fn worker_notifies_offline_only_once() {
        let mut notifier = MockNotifier::new();
        notifier
            .expect_notify_error()
            .withf(|_, e: &AccountError| e.is_offline())
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
        notifier
            .expect_notify_error()
            .withf(|_, e: &AccountError| e.is_offline())
            .times(1)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|_, _| true)
            .times(1)
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
            .expect_notify_error()
            .withf(|_, e: &AccountError| e.is_logged_out())
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
