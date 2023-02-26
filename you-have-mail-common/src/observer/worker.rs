use crate::observer::rpc::ObserverRequest;
use crate::{Account, Notifier, ObserverAccount, ObserverAccountStatus};
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
}

/// Represents and active account.
struct WorkerAccount {
    account: Account,
    status: ObserverAccountStatus,
}

impl Worker {
    pub fn build(
        notifier: Box<dyn Notifier>,
        poll_interval: Duration,
    ) -> (impl Future<Output = ()>, Sender<ObserverRequest>) {
        let (sender, receiver) = proton_api_rs::tokio::sync::mpsc::channel::<ObserverRequest>(5);
        let observer = Self {
            notifier,
            poll_interval,
            accounts: HashMap::new(),
        };
        (observer_task(observer, receiver), sender)
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
        }
    }

    async fn poll_accounts(&mut self) {
        for (email, wa) in &mut self.accounts {
            match wa.account.check().await {
                Ok(check) => {
                    if check.count > 0 {
                        self.notifier.notify(&wa.account, check.count);
                    }
                }
                Err(e) => {
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
