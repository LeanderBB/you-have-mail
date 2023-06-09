use crate::backend::BackendError;
use crate::observer::rpc::ObserverRequest;
use crate::Notification::{AccountTokenRefresh, ProxyApplied};
use crate::{
    Account, AccountError, Config, Notification, Notifier, ObserverAccount, ObserverAccountStatus,
    ObserverError,
};
use crossbeam_channel::{select, tick, Receiver, Sender};
use proton_api_rs::log::{debug, error};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::mem::ManuallyDrop;
use std::sync::Arc;
use std::time::Duration;

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

pub struct BackgroundWorker {
    thread: Option<std::thread::JoinHandle<()>>,
    sender: ManuallyDrop<Sender<ObserverRequest>>,
}

impl BackgroundWorker {
    pub(super) fn send(
        &self,
        request: ObserverRequest,
    ) -> Result<(), crossbeam_channel::SendError<ObserverRequest>> {
        self.sender.send(request)
    }
}

impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        unsafe { ManuallyDrop::drop(&mut self.sender) };
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
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

    pub fn build(notifier: Box<dyn Notifier>, poll_interval: Duration) -> Arc<BackgroundWorker> {
        let (sender, receiver) = crossbeam_channel::bounded::<ObserverRequest>(5);
        let observer = Self::new(notifier, poll_interval);

        let thread = std::thread::spawn(move || {
            observer_task(observer, receiver);
        });

        Arc::new(BackgroundWorker {
            thread: Some(thread),
            sender: ManuallyDrop::new(sender),
        })
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

    fn handle_request(&mut self, request: ObserverRequest) -> bool {
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

                if reply.send(result).is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::LogoutAccount(email, reply) => {
                debug!("Logout account request: account {email}");
                let result = if let Some(account) = self.accounts.get_mut(&email) {
                    let r = account.account.logout().map_err(|e| e.into());
                    if r.is_ok() {
                        account.status = ObserverAccountStatus::LoggedOut;
                    }
                    r
                } else {
                    Err(ObserverError::NoSuchAccount(email))
                };

                if reply.send(result).is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::RemoveAccount(email, reply) => {
                debug!("Remove account request: account {email}");
                let result = if let Some(mut account) = self.accounts.remove(&email) {
                    let r = account.account.logout();
                    if r.is_ok() {
                        self.notifier.notify(Notification::AccountRemoved(&email));
                    }
                    r
                } else {
                    Ok(())
                };

                if reply.send(result.map_err(|e| e.into())).is_err() {
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
                        proxy: v.account.get_proxy().clone(),
                    })
                    .collect::<Vec<_>>();

                if reply.send(Ok(accounts)).is_err() {
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
                let r = Config::store(
                    self.poll_interval,
                    self.accounts.values().map(|a| &a.account),
                );

                if reply.send(r).is_err() {
                    error!("Failed to send reply for gen config request");
                }

                false
            }
            ObserverRequest::SetPollInterval(d) => {
                self.poll_interval = d;
                false
            }
            ObserverRequest::GetPollInterval(reply) => {
                if reply.send(Ok(self.poll_interval)).is_err() {
                    error!("Failed to send reply for poll interval request");
                }
                false
            }
            ObserverRequest::ApplyProxy(email, proxy, reply) => {
                if let Some(p) = &proxy {
                    debug!("Applying new proxy settings email={email} proxy={{protocol={:?} url={}:{} auth={}}}",
                    p.protocol, p.url, p.port,p.auth.is_some()
                    )
                } else {
                    debug!("Applying new proxy settings email={email} proxy=None")
                }

                let result = if let Some(account) = self.accounts.get_mut(&email) {
                    match account.account.set_proxy(proxy.as_ref()) {
                        Ok(changed) => {
                            if changed {
                                self.notifier.notify(ProxyApplied(&email, proxy.as_ref()))
                            }
                            Ok(())
                        }
                        Err(e) => Err(e.into()),
                    }
                } else {
                    Err(ObserverError::NoSuchAccount(email))
                };

                if reply.send(result).is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
            ObserverRequest::GetProxy(email, reply) => {
                debug!("Get proxy settings request email={email}");
                let result = if let Some(account) = self.accounts.get_mut(&email) {
                    Ok(account.account.get_proxy().clone())
                } else {
                    Err(ObserverError::NoSuchAccount(email))
                };

                if reply.send(result).is_err() {
                    error!("Failed to send reply for remove account request");
                }

                false
            }
        }
    }

    fn poll_accounts(&mut self) {
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
            let (result, refreshed) = wa.account.check();
            match result {
                Ok(check) => {
                    if wa.status != ObserverAccountStatus::Online {
                        self.notifier
                            .notify(Notification::AccountOnline(wa.account.email()))
                    }
                    wa.status = ObserverAccountStatus::Online;
                    if !check.emails.is_empty() {
                        self.notifier.notify(Notification::NewEmail {
                            account: wa.account.email(),
                            backend: wa.account.backend().name(),
                            emails: &check.emails,
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
                            BackendError::Timeout(_) | BackendError::Connection(_) => {
                                if wa.status == ObserverAccountStatus::Offline {
                                    return;
                                }
                                self.notifier
                                    .notify(Notification::AccountOffline(wa.account.email()));
                                wa.status = ObserverAccountStatus::Offline;
                            }
                            _ => {
                                wa.status = ObserverAccountStatus::Error;
                                self.notifier
                                    .notify(Notification::AccountError(wa.account.email(), e));
                            }
                        }
                    }
                }
            }
            if refreshed {
                self.notifier
                    .notify(AccountTokenRefresh(wa.account.email()))
            }
        }
    }
}

fn observer_task(mut observer: Worker, receiver: Receiver<ObserverRequest>) {
    debug!("Starting observer loop");
    let mut last_poll_interval = observer.poll_interval;
    let mut ticker = tick(last_poll_interval);
    loop {
        if observer.paused {
            // If the observer is paused we shouldn't wake up all the time to do nothing,
            // wait for the next command to come in to do something.
            if let Ok(request) = receiver.recv() {
                if observer.handle_request(request) {
                    break;
                }
            } else {
                debug!("Receiver closed, exiting loop");
                return;
            }
        } else {
            // Update poll interval
            if last_poll_interval != observer.poll_interval {
                debug!(
                    "Updating observer poll interval old={:?} new={:?}",
                    last_poll_interval, observer.poll_interval
                );
                ticker = tick(observer.poll_interval);
                last_poll_interval = observer.poll_interval;
            }

            select! {
                recv(receiver) -> request => {
                    let Ok(request) = request else {
                        return;
                    };
                    if observer.handle_request(request) {
                        return;
                    }
                }
                recv(ticker) -> _ => {
                    observer.poll_accounts();
                }
            }
        }
    }
    debug!("Exiting observer loop")
}

#[cfg(test)]
mod tests {
    use crate::backend::{BackendError, EmailInfo, MockAccount, NewEmailReply};
    use crate::observer::worker::Worker;
    use crate::{Account, AccountState, MockNotifier, Notification};
    use anyhow::anyhow;
    use mockall::Sequence;
    use std::time::Duration;

    #[test]
    fn worker_notifies_offline_only_once() {
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
            .returning(|| (Err(BackendError::Timeout(anyhow!("offline"))), false));
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
            None,
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // Poll account multiple times
        worker.poll_accounts();
        worker.poll_accounts();
        worker.poll_accounts();
        worker.poll_accounts();
    }

    #[test]
    fn worker_notifies_offline_only_once_and_continues_once_account_comes_online() {
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
            .returning(|| (Err(BackendError::Timeout(anyhow!("offline"))), false));
        mock_account
            .expect_check()
            .times(1)
            .in_sequence(&mut mock_sequence)
            .returning(|| {
                (
                    Ok(NewEmailReply {
                        emails: vec![EmailInfo {
                            sender: "Foo".to_string(),
                            subject: "Bar".to_string(),
                        }],
                    }),
                    false,
                )
            });
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
            None,
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // First two are offline
        worker.poll_accounts();
        worker.poll_accounts();
        // Last one should be online
        worker.poll_accounts();
    }

    #[test]
    fn worker_notifies_logged_out_only_once_and_continues_offline() {
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
            .returning(|| (Err(BackendError::LoggedOut), false));
        mock_account
            .expect_check()
            .times(0)
            .in_sequence(&mut mock_sequence)
            .returning(|| (Ok(NewEmailReply { emails: vec![] }), false));
        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
            None,
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);

        // First all puts the account in logged in mode.
        worker.poll_accounts();
        // Next calls are noop.
        worker.poll_accounts();
        worker.poll_accounts();
    }

    #[test]
    fn worker_notifies_account_refresh() {
        let mut notifier = MockNotifier::new();
        let mut sequence = Sequence::new();
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountError(_, _)))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountTokenRefresh(_)))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountOnline(_)))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::NewEmail { .. }))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());
        notifier
            .expect_notify()
            .withf(|n| matches!(n, Notification::AccountTokenRefresh(_)))
            .times(1)
            .in_sequence(&mut sequence)
            .return_const(());
        notifier.expect_notify().times(0).return_const(());
        let mut mock_account = MockAccount::new();
        let mut mock_sequence = Sequence::new();
        mock_account
            .expect_check()
            .times(1)
            .in_sequence(&mut mock_sequence)
            .returning(|| (Err(BackendError::Unknown(anyhow!("error"))), true));
        mock_account
            .expect_check()
            .times(1)
            .in_sequence(&mut mock_sequence)
            .returning(|| {
                (
                    Ok(NewEmailReply {
                        emails: vec![EmailInfo {
                            sender: "Foo".to_string(),
                            subject: "Bar".to_string(),
                        }],
                    }),
                    true,
                )
            });

        let account = Account::with_state(
            crate::backend::null::new_backend(&[]),
            "foo",
            AccountState::LoggedIn(Box::new(mock_account)),
            None,
        );
        let mut worker = Worker::new(Box::new(notifier), Duration::from_millis(1));

        worker.add_account(account);
        worker.poll_accounts();
        worker.poll_accounts();
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
