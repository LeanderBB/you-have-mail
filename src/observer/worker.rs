use crate::backend::{AccountRefreshedNotifier, AuthRefresher, BackendError, CheckTask};
use crate::observer::stateful_notifier::StatefulNotifier;
use crate::observer::{ObserverError, ObserverResult};
use crate::{AccountError, Config, Notification, Notifier};
use anyhow::anyhow;
use crossbeam_channel::{Receiver, SendError, Sender};
use proton_api_rs::log::{debug, error};
use std::sync::Arc;

pub type TaskList = Vec<Box<dyn CheckTask>>;

struct AccountRefreshedCollector<'a> {
    notifier: &'a StatefulNotifier,
    config: &'a Config,
    accounts: Vec<(String, Box<dyn AuthRefresher>)>,
}

impl<'a> AccountRefreshedNotifier for AccountRefreshedCollector<'a> {
    fn notify_account_refreshed(&mut self, task: &dyn CheckTask) {
        self.accounts
            .push((task.email().to_string(), task.to_refresher()));
    }
}

impl<'a> Drop for AccountRefreshedCollector<'a> {
    fn drop(&mut self) {
        if !self.accounts.is_empty() {
            if let Err(e) = self.config.write(|inner| {
                let accounts = std::mem::take(&mut self.accounts);
                for (email, value) in accounts {
                    inner.account_refreshed(&email, value);
                }
            }) {
                error!("Failed to update config: {e}");
                self.notifier.notify(Notification::ConfigError(e));
            }
        }
    }
}

pub fn poll_inplace(tasks: &[Box<dyn CheckTask>], notifier: &StatefulNotifier, config: &Config) {
    let mut account_refreshed_collector = AccountRefreshedCollector {
        notifier,
        config,
        accounts: Vec::new(),
    };

    for t in tasks {
        debug!("Polling account={} backend={}", t.email(), t.backend_name(),);
        let result = t.check(&mut account_refreshed_collector);
        match result {
            Ok(check) => {
                notifier.account_request_succeed(t.email());
                if !check.emails.is_empty() {
                    notifier.notify(Notification::NewEmail {
                        account: t.email(),
                        backend: t.backend_name(),
                        emails: &check.emails,
                    });
                }
            }
            Err(e) => {
                error!(
                    "Poll failed account={} backend={}: {}",
                    t.email(),
                    t.backend_name(),
                    e
                );
                match e {
                    BackendError::LoggedOut => {
                        notifier.notify(Notification::AccountLoggedOut(t.email()));
                    }
                    BackendError::Timeout(_) | BackendError::Connection(_) => {
                        notifier.notify(Notification::AccountOffline(t.email()));
                    }
                    _ => {
                        notifier.notify(Notification::AccountError(
                            t.email(),
                            AccountError::Backend(e),
                        ));
                    }
                }
            }
        }
    }
}

pub struct TaskRunner(Sender<TaskList>);

impl<T> From<SendError<T>> for ObserverError {
    fn from(e: SendError<T>) -> Self {
        Self::Unknown(anyhow!("Failed to send message to worker: {e}"))
    }
}

impl TaskRunner {
    pub fn new(notifier: Arc<StatefulNotifier>, config: Config) -> std::io::Result<Self> {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let mut background_worker = BackgroundWorker::new(receiver, notifier, config);

        std::thread::Builder::new()
            .name("yhm-worker-thread".to_string())
            .spawn(move || background_worker.run())?;

        Ok(Self(sender))
    }

    pub fn poll(&self, tasks: TaskList) -> ObserverResult<()> {
        self.0.send(tasks)?;
        Ok(())
    }
}

pub struct BackgroundWorker {
    receiver: Receiver<TaskList>,
    notifier: Arc<StatefulNotifier>,
    config: Config,
}

impl BackgroundWorker {
    fn new(receiver: Receiver<TaskList>, notifier: Arc<StatefulNotifier>, config: Config) -> Self {
        Self {
            receiver,
            notifier,
            config,
        }
    }

    fn run(&mut self) {
        loop {
            if let Ok(request) = self.receiver.recv() {
                poll_inplace(&request, self.notifier.as_ref(), &self.config);
            } else {
                debug!("Receiver closed exiting loop");
                return;
            }
        }
    }
}
