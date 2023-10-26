use crate::backend::Backend;
use crate::observer::stateful_notifier::StatefulNotifier;
use crate::observer::worker::{poll_inplace, TaskList, TaskRunner};
use crate::Notification::{AccountAdded, AccountLoggedOut, AccountOnline, AccountRemoved};
use crate::{
    Account, AccountError, Config, ConfigAuthRefresher, ConfigError, Notification, Notifier, Proxy,
};
use anyhow::anyhow;
use proton_api_rs::log::{debug, error, trace};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObserverError {
    #[error("Invalid Poll Mode")]
    InvalidPollMode,
    #[error("No such account {0}")]
    AccountNotFound(String),
    #[error("{0}")]
    AccountError(#[from] AccountError),
    #[error("{0}")]
    Config(#[from] ConfigError),
    #[error("Unknown error occurred: {0}")]
    Unknown(
        #[from]
        #[source]
        anyhow::Error,
    ),
}

pub struct ObserverBuilder {
    notifier: Arc<dyn Notifier>,
    backends: Vec<Arc<dyn Backend>>,
    config: Config,
}

pub type ObserverResult<T> = Result<T, ObserverError>;

impl ObserverBuilder {
    pub fn new(notifier: Arc<dyn Notifier>, config: Config) -> Self {
        Self {
            notifier,
            backends: Default::default(),
            config,
        }
    }

    /// Initialize observer with default list of backends.
    pub fn default_backends(mut self) -> Self {
        self.backends = vec![crate::backend::proton::new_backend()];
        self
    }

    /// Add a backend to the observer.
    pub fn with_backend(mut self, backend: Arc<dyn Backend>) -> Self {
        self.backends.push(backend);
        self
    }

    /// Build the observer, but do no load existing accounts from the config file.
    pub fn build(self, poll_interval: Duration) -> Result<Observer, ObserverError> {
        debug!("Creating observer with emtpy state");
        let mut observer = self.build_without_poll_interval()?;
        observer.set_poll_interval(poll_interval)?;
        Ok(observer)
    }

    fn build_without_poll_interval(self) -> Result<Observer, ObserverError> {
        let notifier = Arc::new(StatefulNotifier::new(self.notifier));
        let worker = TaskRunner::new(notifier.clone(), self.config.clone()).map_err(|e| {
            ObserverError::Unknown(anyhow!("Failed to initialize worker thread {e}"))
        })?;

        Ok(Observer {
            notifier,
            config: self.config,
            accounts: Default::default(),
            worker,
            backends: self.backends,
        })
    }

    /// Build the observer and load existing accounts from a config file.
    pub fn load_from_config(self) -> Result<Observer, ObserverError> {
        debug!("Loading observer from config");
        let mut observer = self.build_without_poll_interval()?;

        let accounts = observer.config.read(|inner| {
            debug!("{} accounts in config", inner.len());
            let mut accounts = Vec::with_capacity(inner.len());
            for (email, cfg) in inner.get_accounts() {
                debug!("Loading account {} ({})", email, cfg.backend);
                let Some(backend) = observer.backend_by_name(&cfg.backend) else {
                    error!(
                        "Could not find backend {} for account {email}. Account will not be added.",
                        cfg.backend
                    );
                    observer.notifier.notify(Notification::Error(format!(
                        "Could not find backend {} for {email}, account will not be added",
                        cfg.backend
                    )));
                    continue;
                };

                match &cfg.auth_refresher {
                    ConfigAuthRefresher::Resolved(r) => {
                        debug!("Loading account from config {} ({})", email, cfg.backend);
                        match backend.account_from_config(cfg.proxy.as_ref(), r.clone()) {
                            Ok(account_state) => {
                                accounts.push(Account::with_state(
                                    backend,
                                    email,
                                    account_state,
                                    cfg.proxy.clone(),
                                ));
                            }
                            Err(e) => {
                                error!("Failed to restore {email} from config: {e}");
                                accounts.push(Account::new(backend, email, cfg.proxy.clone()));
                                observer.notifier.notify(Notification::Error(format!(
                                    "Failed to restore {email}'s form config: {e}"
                                )));
                            }
                        }
                    }
                    ConfigAuthRefresher::None => {
                        debug!("Account {} ({}) has no refresh data", email, cfg.backend);
                        accounts.push(Account::new(backend, email, cfg.proxy.clone()));
                    }
                }
            }
            accounts
        });

        if !accounts.is_empty() {
            debug!("Adding loaded accounts to observer");
            let result = observer
                .config
                .write(|inner| {
                    for account in accounts {
                        inner.add_or_update_account(&account)?;
                        observer.accounts.insert(account.email().into(), account);
                    }
                    Ok(())
                })
                .map_err(|e| {
                    error!("Failed to update config: {e}");
                    observer
                        .notifier
                        .notify(Notification::Error(format!("Failed to update config: {e}")));
                    e
                });

            if let Err(e) = result {
                error!("Failed to update config: {e}");
                observer
                    .notifier
                    .notify(Notification::Error(format!("Failed to update config: {e}")))
            }
        }

        Ok(observer)
    }
}

pub struct Observer {
    backends: Vec<Arc<dyn Backend>>,
    config: Config,
    notifier: Arc<StatefulNotifier>,
    accounts: BTreeMap<String, Account>,
    worker: TaskRunner,
}

impl Observer {
    /// Return number of observed accounts.
    pub fn len(&self) -> usize {
        self.accounts.len()
    }

    /// Return true if no accounts are being observed.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over all the observer accounts.
    pub fn accounts(&self) -> impl Iterator<Item = (&String, &Account)> {
        self.accounts.iter()
    }

    /// Add a new account. If the account already exists, the existing one will be
    /// overwritten.
    pub fn add_account(&mut self, account: Account) -> ObserverResult<()> {
        debug!(
            "Adding account: {} ({})",
            account.email(),
            account.backend().name()
        );
        match self.accounts.entry(account.email().to_string()) {
            Entry::Occupied(mut o) => {
                self.config
                    .write(|inner| inner.add_or_update_account(&account))?;
                self.notifier.notify(AccountOnline(account.email()));
                o.insert(account);
                Ok(())
            }
            Entry::Vacant(v) => {
                self.notifier.notify(AccountAdded(
                    account.email(),
                    account.backend().name(),
                    account.get_proxy().as_ref(),
                ));
                self.config
                    .write(|inner| inner.add_or_update_account(&account))?;
                v.insert(account);
                Ok(())
            }
        }
    }

    /// Log out an account, but does not remove the account from the observer.
    pub fn logout_account(&mut self, email: impl AsRef<str>) -> ObserverResult<()> {
        let email = email.as_ref();
        debug!("Logging out account {}", email);

        let Some(account) = self.accounts.get_mut(email) else {
            return Err(ObserverError::AccountNotFound(email.to_string()));
        };

        account.logout()?;
        self.notifier.notify(AccountLoggedOut(account.email()));
        if let Err(e) = self.config.write(|inner| {
            inner.account_logged_out(account.email());
            Ok(())
        }) {
            error!("Failed to update config after account logout: {e}");
            self.notifier.notify(Notification::Error(format!(
                "Failed to update config after account logout: {e}"
            )));
            return Err(e.into());
        }
        Ok(())
    }

    /// Log out and remove an account from the observer.
    pub fn remove_account(&mut self, email: impl Into<String>) -> ObserverResult<()> {
        let email = email.into();
        debug!("Removing account {}", email);

        match self.accounts.entry(email) {
            Entry::Occupied(o) => {
                let mut account = o.remove();
                self.notifier.notify(AccountRemoved(account.email()));
                if let Err(e) = self.config.write(|inner| {
                    inner.account_removed(account.email());
                    Ok(())
                }) {
                    error!("Failed to update config after account removal: {e}");
                    self.notifier.notify(Notification::Error(format!(
                        "Failed to update config after account removal: {e}"
                    )));
                }

                if let Err(e) = account.logout() {
                    error!("Failed to log out {}: {e}", account.email());
                    return Err(e.into());
                }

                Ok(())
            }
            Entry::Vacant(v) => Err(ObserverError::AccountNotFound(v.key().clone())),
        }
    }

    /// Get an account via email address.
    pub fn get_account(&self, email: impl AsRef<str>) -> Option<&Account> {
        self.accounts.get(email.as_ref())
    }

    /// Get an account via email address.
    pub fn get_account_mut(&mut self, email: impl AsRef<str>) -> Option<&mut Account> {
        self.accounts.get_mut(email.as_ref())
    }

    /// Set poll interval for the observer.
    pub fn set_poll_interval(&mut self, interval: Duration) -> ObserverResult<()> {
        self.config.write(|inner| {
            inner.set_poll_interval(interval);
            Ok(())
        })?;
        Ok(())
    }

    /// Get the observer's current poll interval.
    pub fn get_poll_interval(&self) -> Duration {
        self.config.read(|inner| inner.poll_interval())
    }

    fn collect_task_list(&self) -> ObserverResult<TaskList> {
        let mut tasks = Vec::with_capacity(self.accounts.len());
        for account in self.accounts.values() {
            if let Ok(task) = account.get_task() {
                tasks.push(task);
            }
        }
        Ok(tasks)
    }

    /// Perform account polling on a worker thread.
    pub fn poll(&self) -> ObserverResult<()> {
        trace!("Poll on thread");
        let tasks = self.collect_task_list()?;
        self.worker.poll(tasks)
    }

    /// Perform account polling on this thread.
    pub fn poll_foreground(&self) -> ObserverResult<()> {
        trace!("Poll foreground");
        let tasks = self.collect_task_list()?;
        poll_inplace(&tasks, self.notifier.as_ref(), &self.config);
        Ok(())
    }

    /// Set proxy settings for an account.
    pub fn set_proxy_settings(
        &mut self,
        email: impl AsRef<str>,
        proxy: Option<&Proxy>,
    ) -> ObserverResult<bool> {
        let email = email.as_ref();
        debug!("Setting proxy for {email} is_some={}", proxy.is_some());
        let Some(account) = self.accounts.get_mut(email) else {
            return Err(ObserverError::AccountNotFound(email.to_string()));
        };

        let applied = account.set_proxy(proxy)?;

        if applied {
            self.notifier
                .notify(Notification::ProxyApplied(email, proxy));
            self.config.write(move |inner| {
                inner.account_proxy_changed(email, proxy.cloned());
                Ok(())
            })?;
        }

        Ok(applied)
    }

    /// Get backend by name.
    pub fn backend_by_name(&self, name: impl AsRef<str>) -> Option<Arc<dyn Backend>> {
        let name = name.as_ref();
        self.backends.iter().find(|&b| b.name() == name).cloned()
    }

    /// Get the current configuration.
    pub fn config(&self) -> Config {
        self.config.clone()
    }
}
