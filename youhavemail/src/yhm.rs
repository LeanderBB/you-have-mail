use crate::backend::{Action, Backend, NewEmail, Poller};
use crate::events::Event;
use crate::state::{Account, AccountWatcher, Error as StateError, State};
use secrecy::ExposeSecret;
use sqlite_watcher::watcher::DropRemoveTableObserverHandle;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{Level, debug, error};
use you_have_mail_http::Proxy;

/// Conversion trait for new accounts.
pub trait IntoAccount {
    /// Convert type into an [`Account`].
    ///
    /// # Errors
    ///
    /// Return error if the operation failed.
    fn into_account(self, yhm: &Yhm) -> Result<(), Error>;
}

/// You Have Mail main entry point.
pub struct Yhm {
    state: Arc<State>,
    backends: Vec<Arc<dyn Backend>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Account '{0}' already exists")]
    AccountAlreadyExist(String),
    #[error("Account '{0}' does not exist")]
    AccountNotFound(String),
    #[error("Backend '{0}' does not exist")]
    BackendNotFound(String),
    #[error("Backend: {0}")]
    Backend(#[from] crate::backend::Error),
    #[error("State: {0}")]
    State(#[from] StateError),
}

/// Output of polling an account.
#[derive(Debug)]
pub struct PollOutput {
    /// Email of the account.
    pub email: String,
    /// Backend of the account.
    pub backend: String,
    /// Result of the poll process.
    pub result: crate::backend::Result<Vec<NewEmail>>,
}
impl Yhm {
    /// Create new instance with the given `state` and a default list of backends.
    #[must_use]
    pub fn new(state: Arc<State>) -> Self {
        let backends: [Arc<dyn Backend>; 1] = [crate::backend::proton::Backend::new(None)];
        Self::with_backends(state, backends)
    }

    /// Create new instance with the given `state` and custom list of `backends`.
    pub fn with_backends(
        state: Arc<State>,
        backends: impl IntoIterator<Item = Arc<dyn Backend>>,
    ) -> Self {
        Self {
            state,
            backends: Vec::from_iter(backends),
        }
    }

    /// Poll all active accounts and check for new emails.
    ///
    /// # Errors
    ///
    /// Returns error if the list of accounts can't be loaded from the db. Individual account
    /// errors are returned in the result field.
    #[tracing::instrument(level=Level::DEBUG,skip(self))]
    pub fn poll(&self) -> Result<Vec<PollOutput>, Error> {
        let accounts = self.state.active_accounts()?;
        let mut results = Vec::with_capacity(accounts.len());

        for account in accounts {
            let result = tracing::debug_span!("account", email = account.email()).in_scope(
                || -> Result<PollOutput, Error> {
                    debug!("Polling...");
                    let email = account.email().to_owned();
                    let backend = account.backend().to_owned();
                    let mut account = self.build_account_poller(account)?;

                    Ok(PollOutput {
                        email,
                        backend,
                        result: account.check(),
                    })
                },
            );

            results.push(result?);
        }

        let events = results.iter().map(Event::new).collect::<Vec<_>>();
        self.state.create_or_update_events(&events).map_err(|e| {
            error!("Failed to store result as events: {e}");
            e
        })?;

        Ok(results)
    }

    /// Get the current active backend.
    #[must_use]
    pub fn backends(&self) -> &[Arc<dyn Backend>] {
        &self.backends
    }

    /// Get a backend by `name`
    #[must_use]
    pub fn backend_with_name(&self, name: &str) -> Option<&Arc<dyn Backend>> {
        self.backends.iter().find(|b| b.name() == name)
    }

    /// Get all accounts.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn accounts(&self) -> Result<Vec<Account>, Error> {
        Ok(self.state.accounts().map_err(|e| {
            error!("Failed to retrieve accounts:{e}");
            e
        })?)
    }

    /// Get account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn account(&self, email: &str) -> Result<Option<Account>, Error> {
        Ok(self.state.account(email).map_err(|e| {
            error!("Failed to get account by email: {e}");
            e
        })?)
    }

    /// Returns the number of registered accounts
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn account_count(&self) -> Result<usize, Error> {
        Ok(self.state.account_count()?)
    }

    /// Add a new account to you have mail with `email` and `backend`.
    ///
    /// # Errors
    ///
    /// If the type could not be converted or the db query failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self))]
    pub fn new_account(&self, email: &str, backend: &str) -> Result<Account, Error> {
        tracing::info!("Adding new account");
        if self.backend_with_name(backend).is_none() {
            return Err(Error::BackendNotFound(backend.to_string()));
        };

        Ok(self.state.new_account(email, backend).map_err(|e| {
            error!("Failed to create new account:{e}");
            e
        })?)
    }

    /// Update the `proxy` the account with `email`
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn update_proxy(&self, email: &str, proxy: Option<&Proxy>) -> Result<(), Error> {
        Ok(self.state.set_proxy(email, proxy).map_err(|e| {
            error!("Failed to set proxy for {email}: {e}");
            e
        })?)
    }

    /// Get poll interval.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn poll_interval(&self) -> Result<Duration, Error> {
        Ok(self.state.poll_interval().map_err(|e| {
            error!("Failed to get poll interval:{e}");
            e
        })?)
    }

    /// Set the poll interval.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn set_poll_interval(&self, interval: Duration) -> Result<(), Error> {
        tracing::info!("Poll interval updated: {} Seconds", interval.as_secs());
        Ok(self.state.set_poll_interval(interval).map_err(|e| {
            error!("Failed to set poll interval:{e}");
            e
        })?)
    }

    /// Delete an existing account.
    ///
    /// Logout will be attempted, but if the logout fails the account data will still
    /// be deleted.
    ///
    /// # Errors
    ///
    /// Returns error if the account is not found or if the operation failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self))]
    pub fn delete(&self, email: &str) -> Result<(), Error> {
        tracing::info!("Deleting account");
        let account = self
            .state
            .account(email)?
            .ok_or(Error::AccountNotFound(email.to_owned()))?;

        let mut account = self.build_account_poller(account)?;
        if let Err(e) = account.logout() {
            error!("Failed to logout account: {e}");
        }

        Ok(self.state.delete(email).map_err(|e| {
            error!("Failed to delete account:{e}");
            e
        })?)
    }

    /// Delete an existing account with `email` without logging out first.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self))]
    pub fn delete_without_logout(&self, email: &str) -> Result<(), Error> {
        tracing::info!("Deleting account without logout");
        Ok(self.state.delete(email).map_err(|e| {
            error!("Failed to delete account:{e}");
            e
        })?)
    }

    /// Logout an existing account.
    ///
    /// # Errors
    ///
    /// Returns error if the account is not found or the logout failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self))]
    pub fn logout(&self, email: &str) -> Result<(), Error> {
        tracing::info!("Logging out account");
        let account = self
            .state
            .account(email)?
            .ok_or(Error::AccountNotFound(email.to_owned()))?;

        let mut account = self.build_account_poller(account)?;
        Ok(account.logout().map_err(|e| {
            error!("Failed to logout account:{e}");
            e
        })?)
    }

    /// Apply the given `actions` on the account with `email`.
    ///
    /// # Errors
    ///
    /// Returns error if the action failed to apply.
    pub fn apply_actions(
        &self,
        email: &str,
        actions: impl IntoIterator<Item = Action>,
    ) -> Result<(), Error> {
        let account = self
            .state
            .account(email)?
            .ok_or(Error::AccountNotFound(email.to_owned()))?;

        let mut account = self.build_account_poller(account)?;

        for action in actions {
            account.apply(&action)?;
        }

        Ok(())
    }

    /// Import V1 Configuration and extract all existing accounts.
    ///
    /// # Errors
    ///
    /// Returns errors if the process failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self, config_path))]
    pub fn import_v1(&self, config_path: &Path) -> Result<(), crate::v1::config::Error> {
        tracing::info!("Importing V1 Config");
        let config =
            crate::v1::config::load(self.state.encryption_key().expose_secret(), config_path)
                .map_err(|e| {
                    error!("Failed to load v1 config: {e}");
                    e
                })?;
        let accounts = config.to_v2_accounts();

        if let Some(interval) = config.poll_interval.map(Duration::from_secs) {
            self.state.set_poll_interval(interval).map_err(|e| {
                error!("Failed to set poll interval: {e}");
                e
            })?;
        }

        for account in accounts {
            let new_account = self
                .state
                .new_account(&account.email, &account.backend)
                .map_err(|e| {
                    error!(
                        "Failed to store account '{}'({}): {e}",
                        account.email, account.backend
                    );
                    e
                })?;
            if account.proxy.is_some() {
                new_account.set_proxy(account.proxy.as_ref()).map_err(|e| {
                    error!(
                        "Failed to set account proxy '{}'({}): {e}",
                        account.email, account.backend,
                    );
                    e
                })?;
            }
        }

        Ok(())
    }

    /// Get the last poll events.
    ///
    /// # Errors
    ///
    /// Returns errors if the poll failed.
    pub fn last_events(&self) -> Result<Vec<Event>, Error> {
        Ok(self.state.last_events().map_err(|e| {
            error!("Failed to get last events:{e}");
            e
        })?)
    }

    /// Register a watcher for the accounts table.
    ///
    /// # Errors
    ///
    /// Returns error if we fail to register the observer.
    pub fn watch_accounts<T: AccountWatcher + 'static>(
        &self,
        action: T,
    ) -> Result<DropRemoveTableObserverHandle, Error> {
        Ok(self.state.watch_accounts(action)?)
    }

    fn find_backend(&self, name: &str) -> Option<&Arc<dyn Backend>> {
        self.backends.iter().find(|backend| backend.name() == name)
    }

    /// Construct a new [`Poller`] instance for the given `account`.
    ///
    /// # Errors
    ///
    /// Returns error if we can't find the backend, the client fails to build or there was an
    /// issue processing the account data.
    fn build_account_poller(&self, account: Account) -> crate::backend::Result<Box<dyn Poller>> {
        let Some(backend) = self.find_backend(account.backend()) else {
            return Err(crate::backend::Error::UnknownBackend(
                account.backend().to_owned(),
            ));
        };

        let proxy = account.proxy().inspect_err(|e| {
            error!("Failed to load proxy info from config: {e}");
        })?;
        let client = backend.create_client(proxy).inspect_err(|e| {
            error!("Failed to create client: {e}");
        })?;

        backend.new_poller(client, account).inspect_err(|e| {
            error!("Failed to create poller: {e}");
        })
    }

    /// Access the underlying [`State`] object.
    #[must_use]
    pub fn state(&self) -> &State {
        self.state.as_ref()
    }
}
