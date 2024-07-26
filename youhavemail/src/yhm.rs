use crate::backend::{Backend, NewEmail, Poller};
use crate::state::{Account, Error as StateError, IntoAccount, State};
use http::Proxy;
use secrecy::ExposeSecret;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, Level};

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
    pub fn new(state: Arc<State>) -> Self {
        let state_cloned = Arc::clone(&state);
        Self::with_backends(
            state,
            [crate::backend::proton::new_backend(state_cloned, None)],
        )
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
        let accounts = self.state.accounts()?;
        debug!("Loaded {} accounts", accounts.len());

        let mut results = Vec::with_capacity(accounts.len());

        for account in accounts {
            if account.is_logged_out() {
                debug!("Skipping {} (Logged Out)", account.email());
                continue;
            }

            let result = tracing::debug_span!("account", email = account.email()).in_scope(
                || -> crate::backend::Result<Vec<NewEmail>> {
                    let mut account = self.build_account_poller(&account)?;

                    account.check()
                },
            );

            results.push(PollOutput {
                email: account.email().to_owned(),
                backend: account.backend().to_owned(),
                result,
            })
        }

        Ok(results)
    }

    /// Get the current active backend.
    pub fn backends(&self) -> &[Arc<dyn Backend>] {
        &self.backends
    }

    /// Get a backend by `name`
    pub fn backend_with_name(&self, name: &str) -> Option<&Arc<dyn Backend>> {
        self.backends.iter().find(|b| b.name() == name)
    }

    /// Returns the number of registered accounts
    ///
    /// # Errors
    ///
    /// Returns error if the query failed.
    pub fn account_count(&self) -> Result<usize, Error> {
        Ok(self.state.account_count()?)
    }

    /// Add a new `account` to you have mail.
    ///
    /// New account builders should implement the [`IntoAccount`] trait.
    ///
    /// # Errors
    ///
    /// If the type could not be converted or the db query failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self, account))]
    pub fn add(&self, account: impl IntoAccount) -> Result<(), Error> {
        let account = account
            .into_account(self.state.encryption_key().expose_secret())
            .map_err(|e| {
                error!("Failed to convert into account: {e}");
                e
            })?;
        self.state.store_account(&account).map_err(|e| {
            error!("Failed to store account '{}': {e}", account.email());
            e
        })?;
        Ok(())
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
        Ok(self.state.poll_interval()?)
    }

    /// Set the poll interval.
    ///
    /// # Errors
    ///
    /// Returns error if the operation failed.
    pub fn set_poll_interval(&self, interval: Duration) -> Result<(), Error> {
        Ok(self.state.set_poll_interval(interval)?)
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
        let account = self
            .state
            .account(email)?
            .ok_or(Error::AccountNotFound(email.to_owned()))?;

        let mut account = self.build_account_poller(&account)?;
        if let Err(e) = account.logout() {
            error!("Failed to logout account: {e}");
        }

        Ok(self.state.delete(email)?)
    }

    /// Logout an existing account.
    ///
    /// # Errors
    ///
    /// Returns error if the account is not found or the logout failed.
    #[tracing::instrument(level=Level::DEBUG, skip(self))]
    pub fn logout(&self, email: &str) -> Result<(), Error> {
        let account = self
            .state
            .account(email)?
            .ok_or(Error::AccountNotFound(email.to_owned()))?;

        let mut account = self.build_account_poller(&account)?;
        Ok(account.logout()?)
    }

    #[tracing::instrument(level=Level::DEBUG, skip(self, config_path))]
    pub fn import_v1(&self, config_path: &Path) -> Result<(), crate::v1::config::Error> {
        let config = crate::v1::config::load_v1_config(
            self.state.encryption_key().expose_secret(),
            config_path,
        )
        .map_err(|e| {
            error!("Failed to load v1 config: {e}");
            e
        })?;
        let accounts = config
            .to_v2_accounts(self.state.encryption_key().expose_secret())
            .map_err(|e| {
                error!("Failed to convert into now format: {e}");
                e
            })?;

        if let Some(interval) = config.poll_interval.map(Duration::from_secs) {
            self.state.set_poll_interval(interval).map_err(|e| {
                error!("Failed to set poll interval: {e}");
                e
            })?;
        }

        for account in accounts {
            self.state.store_account(&account).map_err(|e| {
                error!(
                    "Failed to store account '{}'({}): {e}",
                    account.email(),
                    account.backend()
                );
                e
            })?;
        }

        Ok(())
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
    fn build_account_poller(&self, account: &Account) -> crate::backend::Result<Box<dyn Poller>> {
        let Some(backend) = self.find_backend(account.backend()) else {
            return Err(crate::backend::Error::UnknownBackend(
                account.backend().to_owned(),
            ));
        };

        let proxy = account
            .proxy(self.state.encryption_key().expose_secret())
            .map_err(|e| {
                error!("Failed to load proxy info from config");
                e
            })?;
        let client = backend.create_client(proxy).map_err(|e| {
            error!("Failed to create client: {e}");
            e
        })?;

        backend.new_poller(client, account).map_err(|e| {
            error!("Failed to create poller: {e}");
            e
        })
    }
}
